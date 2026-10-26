// 票夹与报销链（PRD §5.16 / §5.11）：
// 发票卡片流（双状态+购方筛查）→ 多选生成报销单 → 付款流水（不涉税）→ 标记已报销
use crate::db::{new_id, now_iso};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct VoucherFull {
    pub id: String,
    pub kind: String,
    pub invoice_code: Option<String>,
    pub invoice_no: Option<String>,
    pub status: String,
    pub book_status: String,
    pub reimburse_status: String,
    pub amount: Option<String>,
    pub issued_on: Option<String>,
    pub seller: Option<String>,
    pub buyer: Option<String>,
    pub expense_category: Option<String>,
    pub buyer_ok: bool, // 购方筛查：与公司档案税号/名匹配或未知
    pub file_path: Option<String>,
    pub has_file: bool,
}

#[derive(Serialize, Deserialize)]
pub struct VoucherInput {
    pub kind: Option<String>,
    pub invoice_code: Option<String>,
    pub invoice_no: String,
    pub amount: Option<String>,
    pub issued_on: Option<String>,
    pub seller: Option<String>,
    pub buyer: Option<String>,
    pub expense_category: Option<String>,
}

#[derive(Serialize)]
pub struct ReimbursementRow {
    pub id: String,
    pub form_date: String,
    pub applicant: Option<String>,
    pub total: String,
    pub status: String,
    pub line_count: i64,
}

pub fn list_vouchers(conn: &Connection) -> rusqlite::Result<Vec<VoucherFull>> {
    // 公司税号/名（筛查基准）
    let co: Option<(String, String)> = conn
        .query_row("SELECT name, tax_no FROM company LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .ok();
    let mut stmt = conn.prepare(
        "SELECT id,kind,invoice_code,invoice_no,status,book_status,reimburse_status,amount,issued_on,seller,buyer,expense_category,file_path
         FROM voucher WHERE status='normal' ORDER BY issued_on DESC, created_at DESC LIMIT 300",
    )?;
    let rows = stmt
        .query_map([], |r| {
            let buyer: Option<String> = r.get(10)?;
            let buyer_ok = match (&buyer, &co) {
                (Some(b), Some((cname, ctax))) => {
                    !b.is_empty() && (cname.contains(b.as_str()) || b.contains(cname.as_str()) || ctax.is_empty())
                }
                _ => true, // 未知不拦，提示由 UI 决定
            };
            Ok(VoucherFull {
                id: r.get(0)?,
                kind: r.get(1)?,
                invoice_code: r.get(2)?,
                invoice_no: r.get(3)?,
                status: r.get(4)?,
                book_status: r.get(5)?,
                reimburse_status: r.get(6)?,
                amount: r.get(7)?,
                issued_on: r.get(8)?,
                seller: r.get(9)?,
                buyer,
                expense_category: r.get(11)?,
                file_path: r.get(12)?,
                has_file: r.get::<_, Option<String>>(12)?.map(|p| !p.is_empty()).unwrap_or(false),
                buyer_ok,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn add_voucher(conn: &Connection, v: VoucherInput) -> rusqlite::Result<String> {
    let id = new_id();
    let now = now_iso();
    conn.execute(
        "INSERT INTO voucher(id,kind,invoice_code,invoice_no,status,book_status,reimburse_status,
           expense_category,amount,issued_on,seller,buyer,is_original,created_at,updated_at)
         VALUES(?1,?2,?3,?4,'normal','unbooked','none',?5,?6,?7,?8,?9,1,?10,?10)",
        rusqlite::params![
            id,
            v.kind.unwrap_or_else(|| "invoice".into()),
            v.invoice_code,
            v.invoice_no,
            v.expense_category,
            v.amount,
            v.issued_on,
            v.seller,
            v.buyer,
            now
        ],
    )?;
    Ok(id)
}

/// 多选发票 → 生成报销单 + 付款流水（tax_status=不涉税）+ 票据置「报销中」
pub fn create_reimbursement(
    conn: &Connection,
    voucher_ids: &[String],
    applicant: Option<String>,
    memo: Option<String>,
) -> rusqlite::Result<String> {
    let rb_id = new_id();
    let now = now_iso();
    let today: String = now.chars().take(10).collect();
    let mut total = 0.0f64;
    for vid in voucher_ids {
        let (amount, inv_no, inv_code): (Option<String>, Option<String>, Option<String>) = conn.query_row(
            "SELECT amount, invoice_no, invoice_code FROM voucher WHERE id=?1 AND reimburse_status='none'",
            [vid],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        let amt: f64 = amount.as_deref().unwrap_or("0").parse().unwrap_or(0.0);
        total += amt;
        conn.execute(
            "INSERT INTO reimbursement_line(id,reimburse_id,expense_on,category,memo,invoice_no,invoice_code,amount)
             VALUES(?1,?2,?3,'票夹归集',?4,?5,?6,?7)",
            rusqlite::params![new_id(), rb_id, today, memo, inv_no, inv_code, format!("{amt:.2}")],
        )?;
        conn.execute("UPDATE voucher SET reimburse_status='in_reimburse' WHERE id=?1", [vid])?;
    }
    conn.execute(
        "INSERT INTO reimbursement(id,form_date,applicant,total,status,source,created_at,updated_at)
         VALUES(?1,?2,?3,?4,'submitted','vault',?5,?5)",
        rusqlite::params![rb_id, today, applicant, format!("{total:.2}"), now],
    )?;
    crate::audit::log_change(
        conn,
        "reimbursement",
        &rb_id,
        "insert",
        &format!("生成报销单 {} 张票 合计{total:.2}", voucher_ids.len()),
    );
    // 反向修正②：公户付款流水（内部往来，不涉税，防双重扣除）
    let acct_id: String = match conn.query_row("SELECT id FROM account WHERE name='公户'", [], |r| r.get(0)) {
        Ok(a) => a,
        Err(_) => {
            let a = new_id();
            conn.execute(
                "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,'公户','bank',1,'0',?2)",
                rusqlite::params![a, now],
            )?;
            a
        }
    };
    conn.execute(
        "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,reimburse_id,created_at,updated_at)
         VALUES(?1,?2,'expense',?3,?4,?5,'none',?6,'报销付款','reimburse','不涉税','in_reimburse','none',?7,?8,?8)",
        rusqlite::params![
            new_id(),
            today,
            format!("{total:.2}"),
            applicant,
            acct_id,
            memo,
            rb_id,
            now
        ],
    )?;
    Ok(rb_id)
}

/// 标记已报销：单据 paid + 票据 reimbursed + 关联事项 reimbursed（paid 后禁 void）
pub fn mark_reimbursed(conn: &Connection, rb_id: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE reimbursement SET status='paid' WHERE id=?1 AND status<>'paid'", [rb_id])?;
    conn.execute(
        "UPDATE voucher SET reimburse_status='reimbursed'
         WHERE id IN (SELECT invoice_no FROM reimbursement_line WHERE reimburse_id=?1 AND invoice_no IS NOT NULL)
            OR id IN (SELECT id FROM voucher WHERE reimburse_status='in_reimburse'
                      AND invoice_no IN (SELECT invoice_no FROM reimbursement_line WHERE reimburse_id=?1))",
        [rb_id],
    )?;
    // 票据侧更稳的路径：凡 in_reimburse 且票号在单据明细中的，置 reimbursed
    conn.execute(
        "UPDATE voucher SET reimburse_status='reimbursed'
         WHERE reimburse_status='in_reimburse'
           AND invoice_no IN (SELECT invoice_no FROM reimbursement_line WHERE reimburse_id=?1 AND invoice_no IS NOT NULL)",
        [rb_id],
    )?;
    conn.execute(
        "UPDATE event SET reimburse_status='reimbursed' WHERE reimburse_id=?1",
        [rb_id],
    )?;
    crate::audit::log_change(conn, "reimbursement", rb_id, "status", "标记已报销");
    Ok(())
}

pub fn list_reimbursements(conn: &Connection) -> rusqlite::Result<Vec<ReimbursementRow>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.form_date, r.applicant, r.total, r.status,
                (SELECT COUNT(*) FROM reimbursement_line l WHERE l.reimburse_id=r.id)
         FROM reimbursement r ORDER BY r.form_date DESC LIMIT 100",
    )?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ReimbursementRow {
                id: r.get(0)?,
                form_date: r.get(1)?,
                applicant: r.get(2)?,
                total: r.get(3)?,
                status: r.get(4)?,
                line_count: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}
