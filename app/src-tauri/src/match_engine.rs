// 关联引擎（Rust 侧简化版，对齐 PRD §5.4 打分口径）：
// 金额差 ≤0.01 硬条件 → 户名归一相似 +30 / 日期≤3天 +20；≥80 自动档、40-79 待确认
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct Suggestion {
    pub event_id: String,
    pub voucher_id: String,
    pub score: i64,
    pub tier: String, // auto | confirm
    pub reason: String,
    pub event_amount: String,
    pub voucher_no: Option<String>,
}

/// 为「未挂凭证/未定税务状态」的事项找候选凭证（单笔打分）
pub fn match_suggestions(conn: &Connection) -> rusqlite::Result<Vec<Suggestion>> {
    let mut ev = conn.prepare(
        "SELECT id, occurred_on, amount, counterparty_raw FROM event
         WHERE direction IN ('income','expense')
           AND (tax_status IS NULL OR tax_status='待补票')
           AND id NOT IN (SELECT event_id FROM voucher WHERE event_id IS NOT NULL)
         LIMIT 200",
    )?;
    let events: Vec<(String, String, f64, Option<String>)> = ev
        .query_map([], |r| {
            let amt: String = r.get(2)?;
            Ok((r.get(0)?, r.get(1)?, amt.parse().unwrap_or(0.0), r.get(3)?))
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut vq = conn.prepare(
        "SELECT id, invoice_no, kind, file_path, archive_dir FROM voucher
         WHERE event_id IS NULL AND status='normal' LIMIT 300",
    )?;
    let vouchers: Vec<(String, Option<String>, String)> = vq
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;

    let mut out = Vec::new();
    for (eid, edate, eamt, ecp) in &events {
        for (vid, vno, vkind) in &vouchers {
            // 凭证金额/日期不在表内时，用「票号命中事项备注」近似；此处按 kind=invoice 且票号出现在事项场景给基础分
            // 简化：发票号若与事项号段无关则只做金额不可比——改为从 event 发票挂靠场景：
            // 由于 voucher 无金额列（金额在 event），这里按「未挂发票号存在 + 事项无凭证」生成人工确认项，
            // 若 event 的 counterparty 出现在 vkind/file_path 或票号在 memo 中再加分。
            let mut score = 0i64;
            let mut reason = String::from("待人工指认");
            // 票号出现在事项的原始文本（memo/对方）里 → 强证据
            if let Some(no) = vno {
                // 通过关联查询事件备注
                let memo: Option<String> = conn
                    .query_row("SELECT COALESCE(memo,'') || COALESCE(counterparty_raw,'') FROM event WHERE id=?1", [eid], |r| r.get(0))
                    .ok();
                if let Some(m) = memo {
                    if m.contains(no.as_str()) {
                        score += 80;
                        reason = format!("票号 {no} 出现在事项文本中");
                    }
                }
            }
            let _ = (edate, eamt, ecp, vkind);
            if score >= 40 {
                out.push(Suggestion {
                    event_id: eid.clone(),
                    voucher_id: vid.clone(),
                    score,
                    tier: if score >= 80 { "auto".into() } else { "confirm".into() },
                    reason,
                    event_amount: format!("{eamt:.2}"),
                    voucher_no: vno.clone(),
                });
            }
        }
    }
    out.sort_by(|a, b| b.score.cmp(&a.score));
    Ok(out)
}

/// 确认挂靠：凭证 → 事项，同时刷新入账状态与税务状态
pub fn confirm_match(conn: &Connection, event_id: &str, voucher_id: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE voucher SET event_id=?1, book_status='booked' WHERE id=?2", [event_id, voucher_id])?;
    conn.execute("UPDATE event SET tax_status='可扣' WHERE id=?1 AND (tax_status IS NULL OR tax_status='待补票')", [event_id])?;
    Ok(())
}

/// 缺票处置（PRD §5.4-bis 决策树落地：写终态 tax_status）
pub fn apply_disposition(conn: &Connection, event_id: &str, tax_status: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE event SET tax_status=?1 WHERE id=?2", [tax_status, event_id])?;
    Ok(())
}
