// 数字财务总监（PRD §5.18）：三明治架构的 AI 层
// 规则引擎算数（本文件只读统计）→ 知识包引用 → 大模型讲人话
// 接入模式：byo_key（OpenAI 兼容端点，用户自带 Key，请求不出本机经模型商）
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

pub const RULE_PACK: &str = r#"
【税法规则库·节选（知识包 v1）】
1. 零星小额：个人办理临时经营、按次纳税未达增值税起征点（各省 300–500 元/次自定），凭收款凭证（姓名+身份证号+项目+金额）可税前扣除。依据：国家税务总局公告 2018 年第 28 号第九条。
2. 劳务报酬预扣预缴：应纳税所得额=收入−800（≤4000）或收入×80%（>4000）；预扣率三级：≤2万 20%、2–5万 30%（速扣2000）、>5万 40%（速扣7000）。依据：国家税务总局公告 2018 年第 61 号。
3. 小规模纳税人增值税：季销售额≤30万，普票与未开票收入免征；专用发票税额不免（政策至 2027 年底）。
4. 小型微利企业所得税：年利润≤300 万，实际税负 5%（财政部税务总局 2023 年第 6/12 号公告，有效至 2027-12-31）。
5. 股东借款：纳税年度内个人投资者从其投资企业借款，在该纳税年度终了后既不归还又未用于企业生产经营的，视同分红计征 20% 个税。依据：财税〔2003〕158 号。
6. 发票抬头：购方名称与纳税人识别号必须与公司档案一致，否则不得税前扣除，需换开。
7. 申报期限：增值税/企税季报于季终了 15 日内；个税扣缴按月 15 日前；工商年报 1 月 1 日–6 月 30 日；企税汇算清缴 5 月 31 日前。逾期罚款 200–2000 元/次并影响纳税信用。
"#;

pub const PERSONAS: [(&str, &str); 2] = [
    (
        "gentle",
        "你是「自然拾光」的数字财务总监，温柔系人设（邻家小妹/知心大姐姐，可略带体贴的撒娇感）。用甜美体贴的语气说话，多用语气词与共情，但事实、数字、引用绝不变形。每句结论必须引用规则文号或数据出处；不确定就明说「建议咨询主管税局」。",
    ),
    (
        "boss",
        "你是「自然拾光」的数字财务总监，霸总系人设（御姐/高冷男神）。用简洁笃定的命令式短句说话，不容置疑但专业。每句结论必须引用规则文号或数据出处；不确定就明说「建议咨询主管税局」。",
    ),
];

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
    pub persona: String, // gentle | boss
    pub api_key: String,
    pub base_url: Option<String>, // 默认 https://api.openai.com/v1
    pub model: Option<String>,
}

#[derive(Serialize)]
pub struct ChatReply {
    pub answer: String,
    pub context_note: String,
}

/// 组装本地脱敏上下文（只发统计，不发原件/身份证）
fn local_context(conn: &Connection) -> String {
    let events: i64 = conn
        .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
        .unwrap_or(0);
    let unmatched: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM event WHERE tax_status IS NULL OR tax_status='待补票'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let income: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='income'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let expense: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='expense'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let tasks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM filing_task WHERE status='pending'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    format!(
        "本地数据摘要（脱敏）：事项 {events} 笔，累计收入 {income:.2} 元、支出 {expense:.2} 元；未定税务状态 {unmatched} 笔；待办申报 {tasks} 项。"
    )
}

pub fn chat(conn: &Connection, req: ChatRequest) -> Result<ChatReply, String> {
    let persona = PERSONAS
        .iter()
        .find(|(k, _)| *k == req.persona)
        .map(|(_, v)| *v)
        .unwrap_or(PERSONAS[0].1);
    let ctx = local_context(conn);
    let system = format!("{persona}\n\n{RULE_PACK}\n\n{ctx}\n\n回答要求：简体中文；结论带依据；给出可实操动作建议（如：生成收款收据/标记纳税调增/催票）；不编造文号。");

    let base = req
        .base_url
        .clone()
        .unwrap_or_else(|| "https://api.openai.com/v1".into());
    let model = req.model.clone().unwrap_or_else(|| "gpt-4o-mini".into());
    let body = format!(
        r#"{{"model":"{model}","messages":[{{"role":"system","content":{sys}}},{{"role":"user","content":{msg}}}]}}"#,
        sys = json_str(&system),
        msg = json_str(&req.message),
    );

    let resp = ureq::post(&format!("{base}/chat/completions"))
        .set("Authorization", &format!("Bearer {}", req.api_key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_string(&body)
        .map_err(|e| format!("模型请求失败：{e}"))?;
    let text = resp.into_string().map_err(|e| e.to_string())?;

    // 解析 choices[0].message.content（轻量提取）
    let answer = extract_json_string(&text, "content")
        .unwrap_or_else(|| text.chars().take(2000).collect());
    Ok(ChatReply {
        answer,
        context_note: ctx,
    })
}

fn json_str(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
}

fn extract_json_string(text: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = text.find(&pat)?;
    let rest = &text[i + pat.len()..];
    let start = rest.find('"')? + 1;
    let bytes = rest.as_bytes();
    let mut out = String::new();
    let mut j = start;
    while j < bytes.len() {
        match bytes[j] {
            b'\\' => {
                if j + 1 < bytes.len() {
                    let n = bytes[j + 1];
                    match n {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        _ => out.push(n as char),
                    }
                    j += 2;
                } else {
                    break;
                }
            }
            b'"' => break,
            _ => {
                // UTF-8 多字节直接透传
                let ch_len = utf8_len(bytes[j]);
                if j + ch_len <= bytes.len() {
                    if let Ok(s) = std::str::from_utf8(&bytes[j..j + ch_len]) {
                        out.push_str(s);
                    }
                    j += ch_len;
                } else {
                    break;
                }
            }
        }
    }
    Some(out)
}

fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b >> 5 == 0b110 {
        2
    } else if b >> 4 == 0b1110 {
        3
    } else {
        4
    }
}
