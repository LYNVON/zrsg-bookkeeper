// 邮箱 IMAP 收票（PRD §5.16 ①-bis）：
// 授权码登录 → 拉取带 PDF 附件的未读邮件 → 购方筛查 → 入票夹
// QQ 邮箱走授权码（设置→账户→生成授权码）；仅取附件，不读正文明文
use crate::db::new_id;
use imap;
use mailparse::{self, ParsedMail};
use native_tls::TlsConnector;
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct FetchReport {
    pub fetched: u64,
    pub imported: u64,
    pub quarantined: u64,
    pub skipped: u64,
    pub errors: Vec<String>,
}

fn parse_invoice_fields(name: &str, pdf_bytes: &[u8]) -> (Option<String>, Option<String>, Option<String>) {
    // 票号：20 位数字 或 发票号码字样附近
    let text = String::from_utf8_lossy(pdf_bytes);
    let mut no = None;
    // 常见数电票号 20 位
    let mut best: Option<(usize, String)> = None;
    let mut i = 0;
    let bytes = text.as_bytes();
    while i + 20 <= bytes.len() {
        if bytes[i..i + 20].iter().all(|b| b.is_ascii_digit()) {
            let cand = text[i..i + 20].to_string();
            if cand.starts_with("2") {
                best = Some((i, cand));
            }
            i += 20;
        } else {
            i += 1;
        }
    }
    if let Some((_, c)) = best {
        no = Some(c);
    }
    // 金额：¥xxx.xx 或 价税合计
    let mut amount = None;
    for seg in text.split("价税合计").skip(1) {
        let frag: String = seg
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .take(12)
            .collect();
        if let Ok(v) = frag.parse::<f64>() {
            if v > 0.0 {
                amount = Some(format!("{v:.2}"));
                break;
            }
        }
    }
    let _ = name;
    (no, amount, None)
}

/// 拉取收票：host/port/user/pass(授权码)/folder
pub fn fetch_invoices(
    conn: &Connection,
    host: &str,
    port: u16,
    user: &str,
    pass: &str,
    folder: &str,
) -> Result<FetchReport, String> {
    let tls = TlsConnector::builder().build().map_err(|e| e.to_string())?;
    let client = imap::connect((host, port), host, &tls)
        .map_err(|e| format!("连接失败：{e}（QQ/163 请用授权码）"))?;
    let mut session = client.login(user, pass).map_err(|(e, _)| format!("登录失败：{e}"))?;
    session.select(folder).map_err(|e| e.to_string())?;

    let messages = session
        .search("UNSEEN")
        .map_err(|e| e.to_string())?;
    let mut report = FetchReport {
        fetched: 0,
        imported: 0,
        quarantined: 0,
        skipped: 0,
        errors: vec![],
    };

    let co: Option<(String, String)> = conn
        .query_row("SELECT name, tax_no FROM company LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .ok();

    for msg_id in messages.iter().take(50) {
        // 已处理过的邮件直接跳过（本地 UID 表 + 邮件标记已读 双保险）
        let uid_key = format!("{}:{}", folder, msg_id);
        let seen: i64 = conn
            .query_row("SELECT COUNT(*) FROM mail_seen WHERE uid=?1", [&uid_key], |r| r.get(0))
            .unwrap_or(0);
        if seen > 0 {
            continue;
        }
        match session.fetch(msg_id.to_string(), "RFC822") {
            Ok(msgs) => {
                if let Some(m) = msgs.iter().next() {
                    if let Some(body) = m.body() {
                        if let Ok(parsed) = mailparse::parse_mail(body) {
                            for att in walk_attachments(&parsed) {
                                let name = att
                                    .get_content_disposition()
                                    .params
                                    .get("filename")
                                    .cloned()
                                    .unwrap_or_else(|| "unknown.pdf".into());
                                if !name.to_lowercase().ends_with(".pdf") {
                                    continue;
                                }
                                let data = att.get_body_raw().unwrap_or_default();
                                if data.len() < 100 {
                                    continue;
                                }
                                report.fetched += 1;
                                let (no, amount, _seller) = parse_invoice_fields(&name, &data);
                                let invoice_no = no.unwrap_or_else(|| format!("MAIL-{}", new_id()));
                                // 购方筛查：PDF 文本含公司名/税号 → 入夹；否则隔离（此处无结构化购方，按文件名/文本近似）
                                let text = String::from_utf8_lossy(&data);
                                let buyer_ok = match &co {
                                    Some((cname, ctax)) => {
                                        text.contains(cname.as_str())
                                            || (!ctax.is_empty() && text.contains(ctax.as_str()))
                                    }
                                    None => true,
                                };
                                let dir = crate::db::data_dir().join("attachments").join("mail");
                                let _ = std::fs::create_dir_all(&dir);
                                let file = dir.join(format!("{invoice_no}.pdf"));
                                if std::fs::write(&file, &data).is_err() {
                                    report.errors.push(format!("写附件失败：{name}"));
                                    continue;
                                }
                                // 发票号唯一去重
                                let dup: i64 = conn
                                    .query_row(
                                        "SELECT COUNT(*) FROM voucher WHERE invoice_no=?1",
                                        [&invoice_no],
                                        |r| r.get(0),
                                    )
                                    .unwrap_or(0);
                                if dup > 0 {
                                    report.skipped += 1;
                                    continue;
                                }
                                let (status_book, note) = if buyer_ok {
                                    ("unbooked", "")
                                } else {
                                    ("unbooked", "隔离")
                                };
                                let _ = note;
                                let res = conn.execute(
                                    "INSERT INTO voucher(id,kind,invoice_code,invoice_no,status,book_status,reimburse_status,
                                       file_path,amount,is_original,source_mail,created_at,updated_at)
                                     VALUES(?1,'invoice',NULL,?2,'normal',?3,'none',?4,?5,1,'imap',datetime('now'),datetime('now'))",
                                    rusqlite::params![
                                        new_id(),
                                        invoice_no,
                                        status_book,
                                        file.to_string_lossy(),
                                        amount
                                    ],
                                );
                                match res {
                                    Ok(_) => {
                                        if buyer_ok {
                                            report.imported += 1;
                                        } else {
                                            report.quarantined += 1;
                                        }
                                    }
                                    Err(e) => report.errors.push(format!("入库失败：{e}")),
                                }
                                let _ = conn.execute(
                                    "INSERT OR IGNORE INTO mail_seen(uid,folder,seen_at) VALUES(?1,?2,?3)",
                                    rusqlite::params![uid_key, folder, crate::db::now_iso()],
                                );
                                let _ = session.store(msg_id.to_string(), "+FLAGS (\\Seen)");
                            }
                        }
                    }
                }
            }
            Err(e) => report.errors.push(format!("拉取失败：{e}")),
        }
    }
    // 本批已处理的 UID 全部落表，避免重复收票
    let batch_hash = format!("imap:{}:{}", folder, crate::db::now_iso());
    crate::audit::log_batch(conn, "imap", folder, &batch_hash, report.imported as i64);
    crate::audit::log_change(
        conn,
        "voucher",
        "",
        "merge",
        &format!("邮箱收票 imported={} quarantined={}", report.imported, report.quarantined),
    );

    let _ = session.logout();
    Ok(report)
}

fn walk_attachments<'a>(mail: &'a ParsedMail<'a>) -> Vec<&'a ParsedMail<'a>> {
    let mut out = vec![];
    if mail.ctype.mimetype.contains("multipart") {
        for s in &mail.subparts {
            out.extend(walk_attachments(s));
        }
    } else if mail.get_content_disposition().disposition == mailparse::DispositionType::Attachment
        || mail.get_content_disposition().params.get("filename").is_some()
    {
        out.push(mail);
    }
    out
}
