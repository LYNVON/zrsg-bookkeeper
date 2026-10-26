// 外部报销单识别（PRD §5.11 入口 C）：PDF 文本抽取 → 发票号/金额硬匹配票夹 → 生成报销单
use crate::vault;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct DocParseResult {
    pub invoice_nos: Vec<String>,
    pub amounts: Vec<String>,
    pub matched: u64,
    pub unmatched: u64,
    pub reimburse_id: Option<String>,
    pub hint: String,
}

pub fn parse_external_doc(
    conn: &Connection,
    path: &str,
    applicant: Option<String>,
) -> Result<DocParseResult, String> {
    let text = pdf_extract::extract_text(path).map_err(|e| format!("PDF 解析失败：{e}"))?;
    // 发票号：20 位数字（以 2 开头）
    let mut nos = Vec::new();
    let b = text.as_bytes();
    let mut i = 0;
    while i + 20 <= b.len() {
        if b[i..i + 20].iter().all(|c| c.is_ascii_digit()) {
            let cand = text[i..i + 20].to_string();
            if cand.starts_with('2') {
                nos.push(cand);
            }
            i += 20;
        } else {
            i += 1;
        }
    }
    nos.sort();
    nos.dedup();

    // 金额：数字.两位（粗提取）
    let mut amounts = Vec::new();
    for seg in text.split(|c: char| c == ' ' || c == '\n' || c == '\t') {
        let t = seg.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
        if t.contains('.') && t.len() >= 4 {
            if let Ok(v) = t.parse::<f64>() {
                if (0.01..1_000_000.0).contains(&v) {
                    amounts.push(format!("{v:.2}"));
                }
            }
        }
    }

    // 票夹硬匹配
    let mut matched_ids = Vec::new();
    let mut matched = 0u64;
    let mut unmatched = 0u64;
    for no in &nos {
        let vid: Option<String> = conn
            .query_row(
                "SELECT id FROM voucher WHERE invoice_no=?1 AND reimburse_status='none'",
                [no],
                |r| r.get(0),
            )
            .ok();
        match vid {
            Some(id) => {
                matched_ids.push(id);
                matched += 1;
            }
            None => unmatched += 1,
        }
    }

    let mut reimburse_id = None;
    if !matched_ids.is_empty() {
        let id = vault::create_reimbursement(conn, &matched_ids, applicant, Some("外部报销单识别".into()))
            .map_err(|e| e.to_string())?;
        reimburse_id = Some(id);
    }

    Ok(DocParseResult {
        invoice_nos: nos,
        amounts: amounts.into_iter().take(10).collect(),
        matched,
        unmatched,
        reimburse_id,
        hint: if matched == 0 {
            "未识别到票夹中的发票号——可手动填写报销单".into()
        } else {
            format!("匹配 {matched} 张发票并已生成报销单草稿；未匹配 {unmatched} 个号码请人工指认")
        },
    })
}
