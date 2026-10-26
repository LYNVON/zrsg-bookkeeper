// Tauri 命令层：记账 / 事项 / 申报任务 / 公司档案 / 备份 / CSV 导入
use crate::db::{new_id, now_iso, DbState};
use rusqlite::Connection;
use sha2::Digest;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DbStatus {
    pub ok: bool,
    pub db_path: String,
    pub company: Option<String>,
    pub event_count: i64,
    pub voucher_count: i64,
    pub task_pending: i64,
}

#[derive(Serialize, Deserialize)]
pub struct EventInput {
    pub occurred_on: String,
    pub direction: String,
    pub amount: String,
    pub counterparty: Option<String>,
    pub memo: Option<String>,
    pub category: Option<String>,
    pub tax_status: Option<String>,
    pub account_name: Option<String>,
}

#[derive(Serialize)]
pub struct EventRow {
    pub id: String,
    pub occurred_on: String,
    pub direction: String,
    pub amount: String,
    pub counterparty: Option<String>,
    pub memo: Option<String>,
    pub category: Option<String>,
    pub tax_status: Option<String>,
    pub source: String,
}

#[derive(Serialize)]
pub struct TaskRow {
    pub id: String,
    pub title: String,
    pub due_on: String,
    pub status: String,
    pub channel: Option<String>,
}

#[derive(Serialize)]
pub struct Dashboard {
    pub company: Option<String>,
    pub event_count: i64,
    pub month_income: String,
    pub month_expense: String,
    pub tasks: Vec<TaskRow>,
    pub unmatched: i64,
}

#[derive(Serialize, Deserialize)]
pub struct CompanyInput {
    pub name: String,
    pub tax_no: String,
    pub legal_person: Option<String>,
    pub register_city: Option<String>,
    pub tax_city: Option<String>,
    pub bank_name: Option<String>,
    pub bank_account: Option<String>,
}

fn with_db<T>(db: &DbState, f: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Result<T, String> {
    let guard = db.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    f(conn).map_err(|e| e.to_string())
}

fn read_status(conn: &Connection, path: &std::path::Path) -> rusqlite::Result<DbStatus> {
    let company = conn
        .query_row("SELECT name FROM company LIMIT 1", [], |r| r.get::<_, String>(0))
        .ok();
    let event_count = conn.query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))?;
    let voucher_count = conn.query_row("SELECT COUNT(*) FROM voucher", [], |r| r.get(0))?;
    let task_pending = conn
        .query_row("SELECT COUNT(*) FROM filing_task WHERE status='pending'", [], |r| r.get(0))?;
    Ok(DbStatus {
        ok: true,
        db_path: path.display().to_string(),
        company,
        event_count,
        voucher_count,
        task_pending,
    })
}

#[tauri::command]
pub fn init_app(state: tauri::State<'_, DbState>) -> Result<DbStatus, String> {
    let path = crate::db::resolve_db_path();
    let conn = crate::cryptdb::open_app_db(&path)?;
    let status = read_status(&conn, &path).map_err(|e| e.to_string())?;
    // 播种标准申报任务（已有则跳过）
    let _ = crate::report::seed_default_tasks(&conn);
    let mut guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(status)
}

#[tauri::command]
pub fn db_ping(state: tauri::State<'_, DbState>) -> Result<DbStatus, String> {
    with_db(state.inner(), |c| read_status(c, std::path::Path::new("data/zrsj.db")))
}

#[tauri::command]
pub fn add_event(state: tauri::State<'_, DbState>, input: EventInput) -> Result<String, String> {
    let id = new_id();
    let now = now_iso();
    with_db(state.inner(), |c| {
        let acct_name = input.account_name.unwrap_or_else(|| "公户".into());
        let acct_id: Option<String> = c
            .query_row("SELECT id FROM account WHERE name=?1", [&acct_name], |r| r.get(0))
            .ok();
        let acct_id = match acct_id {
            Some(a) => a,
            None => {
                let a = new_id();
                c.execute(
                    "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,?2,'bank',1,'0',?3)",
                    rusqlite::params![a, acct_name, now],
                )?;
                a
            }
        };
        c.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,'none',?7,?8,'manual',?9,'none','none',?10,?10)",
            rusqlite::params![
                id,
                input.occurred_on,
                input.direction,
                input.amount,
                input.counterparty,
                acct_id,
                input.memo,
                input.category,
                input.tax_status,
                now
            ],
        )?;
        Ok(id)
    })
}

#[tauri::command]
pub fn list_events(state: tauri::State<'_, DbState>, limit: i64) -> Result<Vec<EventRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,occurred_on,direction,amount,counterparty_raw,memo,category,tax_status,source
             FROM event ORDER BY occurred_on DESC, created_at DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit], |r| {
                Ok(EventRow {
                    id: r.get(0)?,
                    occurred_on: r.get(1)?,
                    direction: r.get(2)?,
                    amount: r.get(3)?,
                    counterparty: r.get(4)?,
                    memo: r.get(5)?,
                    category: r.get(6)?,
                    tax_status: r.get(7)?,
                    source: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn list_tasks(state: tauri::State<'_, DbState>) -> Result<Vec<TaskRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare("SELECT id,title,due_on,status,channel FROM filing_task ORDER BY due_on LIMIT 100")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(TaskRow {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    due_on: r.get(2)?,
                    status: r.get(3)?,
                    channel: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn toggle_task(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    with_db(state.inner(), |c| {
        c.execute(
            "UPDATE filing_task SET status = CASE WHEN status='done' THEN 'pending' ELSE 'done' END,
             done_at = CASE WHEN status='done' THEN NULL ELSE ?2 END WHERE id=?1",
            rusqlite::params![id, now_iso()],
        )?;
        Ok(())
    })
}

fn ymd_since(days: u64) -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut d = (t.saturating_sub(days * 86400) / 86400) as i64;
    let mut y = 1970i64;
    loop {
        let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
        let len = if leap { 366 } else { 365 };
        if d < len {
            break;
        }
        d -= len;
        y += 1;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let mdays = [31i64, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0;
    while m < 12 && d >= mdays[m] {
        d -= mdays[m];
        m += 1;
    }
    format!("{y:04}-{:02}-{:02}", m + 1, d + 1)
}

#[tauri::command]
pub fn dashboard(state: tauri::State<'_, DbState>) -> Result<Dashboard, String> {
    with_db(state.inner(), |c| {
        let company = c
            .query_row("SELECT name FROM company LIMIT 1", [], |r| r.get::<_, String>(0))
            .ok();
        let event_count = c.query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))?;
        let since = ymd_since(30);
        let month_income: String = c.query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='income' AND occurred_on>=?1",
            [&since],
            |r| Ok(format!("{:.2}", r.get::<_, f64>(0)?)),
        )?;
        let month_expense: String = c.query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='expense' AND occurred_on>=?1",
            [&since],
            |r| Ok(format!("{:.2}", r.get::<_, f64>(0)?)),
        )?;
        let mut stmt = c.prepare(
            "SELECT id,title,due_on,status,channel FROM filing_task WHERE status='pending' ORDER BY due_on LIMIT 5",
        )?;
        let tasks = stmt
            .query_map([], |r| {
                Ok(TaskRow {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    due_on: r.get(2)?,
                    status: r.get(3)?,
                    channel: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let unmatched: i64 = c.query_row(
            "SELECT COUNT(*) FROM event WHERE tax_status IS NULL OR tax_status='待补票'",
            [],
            |r| r.get(0),
        )?;
        Ok(Dashboard {
            company,
            event_count,
            month_income,
            month_expense,
            tasks,
            unmatched,
        })
    })
}

#[tauri::command]
pub fn save_company(state: tauri::State<'_, DbState>, input: CompanyInput) -> Result<(), String> {
    let now = now_iso();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO company(id,name,tax_no,legal_person,register_city,tax_city,bank_name,bank_account,taxpayer_type,filing_cycle,created_at,updated_at)
             VALUES('main',?1,?2,?3,?4,?5,?6,?7,'small','quarter',?8,?8)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name,tax_no=excluded.tax_no,
               legal_person=excluded.legal_person,register_city=excluded.register_city,
               tax_city=excluded.tax_city,bank_name=excluded.bank_name,
               bank_account=excluded.bank_account,updated_at=excluded.updated_at",
            rusqlite::params![
                input.name,
                input.tax_no,
                input.legal_person,
                input.register_city,
                input.tax_city,
                input.bank_name,
                input.bank_account,
                now
            ],
        )?;
        Ok(())
    })
}

#[tauri::command]
pub fn get_company(state: tauri::State<'_, DbState>) -> Result<Option<CompanyInput>, String> {
    with_db(state.inner(), |c| {
        match c.query_row(
            "SELECT name,tax_no,legal_person,register_city,tax_city,bank_name,bank_account FROM company LIMIT 1",
            [],
            |r| {
                Ok(CompanyInput {
                    name: r.get(0)?,
                    tax_no: r.get(1)?,
                    legal_person: r.get(2)?,
                    register_city: r.get(3)?,
                    tax_city: r.get(4)?,
                    bank_name: r.get(5)?,
                    bank_account: r.get(6)?,
                })
            },
        ) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e),
        }
    })
}

#[tauri::command]
pub fn backup_now(state: tauri::State<'_, DbState>) -> Result<String, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    let dir = crate::db::data_dir().join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join(format!("zrsj-{}.db", now_iso()));
    conn.execute(
        &format!("VACUUM INTO '{}'", out.display().to_string().replace('\'', "''")),
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

fn parse_f(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
        .collect();
    if t.is_empty() || t == "-" {
        None
    } else {
        t.parse().ok()
    }
}

// ── 关联工作台（v1）：清单 / 匹配建议 / 确认 / 处置 ──────────

#[derive(Serialize)]
pub struct UnmatchedEvent {
    pub id: String,
    pub occurred_on: String,
    pub direction: String,
    pub amount: String,
    pub counterparty: Option<String>,
    pub memo: Option<String>,
    pub tax_status: Option<String>,
}

#[tauri::command]
pub fn list_unmatched(state: tauri::State<'_, DbState>) -> Result<Vec<UnmatchedEvent>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,occurred_on,direction,amount,counterparty_raw,memo,tax_status FROM event
             WHERE direction IN ('income','expense')
               AND (tax_status IS NULL OR tax_status='待补票')
             ORDER BY occurred_on DESC LIMIT 200",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(UnmatchedEvent {
                    id: r.get(0)?,
                    occurred_on: r.get(1)?,
                    direction: r.get(2)?,
                    amount: r.get(3)?,
                    counterparty: r.get(4)?,
                    memo: r.get(5)?,
                    tax_status: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn match_suggestions(state: tauri::State<'_, DbState>) -> Result<Vec<crate::match_engine::Suggestion>, String> {
    with_db(state.inner(), |c| crate::match_engine::match_suggestions(c))
}

#[tauri::command]
pub fn confirm_match(state: tauri::State<'_, DbState>, event_id: String, voucher_id: String) -> Result<(), String> {
    with_db(state.inner(), |c| {
        crate::match_engine::confirm_match(c, &event_id, &voucher_id)?;
        // 规则沉淀：确认行为 → 同对方规则（建议模式，默认不自动）
        let _ = crate::rules::learn_from_confirm(c, &event_id, "可扣");
        Ok(())
    })
}

#[tauri::command]
pub fn apply_disposition(state: tauri::State<'_, DbState>, event_id: String, tax_status: String) -> Result<(), String> {
    with_db(state.inner(), |c| crate::match_engine::apply_disposition(c, &event_id, &tax_status))
}

/// 7 天内到期的申报任务（桌面通知数据源）
#[derive(Serialize)]
pub struct DueTask {
    pub id: String,
    pub title: String,
    pub due_on: String,
    pub days_left: i64,
}

#[tauri::command]
pub fn due_tasks(state: tauri::State<'_, DbState>) -> Result<Vec<DueTask>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,title,due_on FROM filing_task WHERE status='pending' ORDER BY due_on LIMIT 20",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, title, due)| {
                let days = days_until(&due);
                if days <= 7 {
                    Some(DueTask { id, title, due_on: due, days_left: days })
                } else {
                    None
                }
            })
            .collect())
    })
}

fn days_until(due: &str) -> i64 {
    let p: Vec<i64> = due
        .split('-')
        .map(|x| x.chars().filter(|c| c.is_ascii_digit()).collect::<String>())
        .filter_map(|x| x.parse().ok())
        .collect();
    if p.len() != 3 {
        return 999;
    }
    let today = ymd_since(0);
    let t: Vec<i64> = today.split('-').filter_map(|x| x.parse().ok()).collect();
    if t.len() != 3 {
        return 999;
    }
    (p[0] - t[0]) * 365 + (p[1] - t[1]) * 30 + (p[2] - t[2])
}

// ── 票夹与报销链（v1.9 / Phase 2a）─────────────────────────

#[tauri::command]
pub fn list_vouchers(state: tauri::State<'_, DbState>) -> Result<Vec<crate::vault::VoucherFull>, String> {
    with_db(state.inner(), |c| crate::vault::list_vouchers(c))
}

#[tauri::command]
pub fn add_voucher(state: tauri::State<'_, DbState>, input: crate::vault::VoucherInput) -> Result<String, String> {
    with_db(state.inner(), |c| crate::vault::add_voucher(c, input))
}

#[tauri::command]
pub fn create_reimbursement(
    state: tauri::State<'_, DbState>,
    voucher_ids: Vec<String>,
    applicant: Option<String>,
    memo: Option<String>,
) -> Result<String, String> {
    with_db(state.inner(), |c| crate::vault::create_reimbursement(c, &voucher_ids, applicant, memo))
}

#[tauri::command]
pub fn mark_reimbursed(state: tauri::State<'_, DbState>, rb_id: String) -> Result<(), String> {
    with_db(state.inner(), |c| crate::vault::mark_reimbursed(c, &rb_id))
}

#[tauri::command]
pub fn list_reimbursements(state: tauri::State<'_, DbState>) -> Result<Vec<crate::vault::ReimbursementRow>, String> {
    with_db(state.inner(), |c| crate::vault::list_reimbursements(c))
}

/// 导出打印文档：写 data/exports/<name>.html 并用系统默认浏览器打开（打印/存 PDF）
#[tauri::command]
pub fn export_document(name: String, html: String) -> Result<String, String> {
    let dir = crate::db::data_dir().join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let out = dir.join(format!("{safe}.html"));
    std::fs::write(&out, html).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

#[derive(Serialize)]
pub struct RbLineRow {
    pub expense_on: Option<String>,
    pub category: Option<String>,
    pub invoice_no: Option<String>,
    pub amount: String,
}

#[tauri::command]
pub fn reimbursement_lines(state: tauri::State<'_, DbState>, rb_id: String) -> Result<Vec<RbLineRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT expense_on, category, invoice_no, amount FROM reimbursement_line WHERE reimburse_id=?1",
        )?;
        let rows = stmt
            .query_map([rb_id], |r| {
                Ok(RbLineRow {
                    expense_on: r.get(0)?,
                    category: r.get(1)?,
                    invoice_no: r.get(2)?,
                    amount: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

// ── 税费报表与季报底稿（T23）─────────────────────────────

#[tauri::command]
pub fn tax_report(state: tauri::State<'_, DbState>, start: String, end: String) -> Result<crate::tax::TaxReport, String> {
    with_db(state.inner(), |c| crate::tax::tax_report(c, &start, &end))
}

#[tauri::command]
pub fn export_tax_draft(state: tauri::State<'_, DbState>, start: String, end: String) -> Result<String, String> {
    let html = with_db(state.inner(), |c| {
        let r = crate::tax::tax_report(c, &start, &end)?;
        let co = c
            .query_row("SELECT name FROM company LIMIT 1", [], |x| x.get::<_, String>(0))
            .unwrap_or_else(|_| "未建档".into());
        let html = crate::tax::render_draft_html(&r, &co);
        let amounts = serde_json::to_string(&r).unwrap_or_default();
        c.execute(
            "INSERT INTO filing_draft(id,period,tax_type,amounts,calculation_trace,created_on)
             VALUES(?1,?2,'quarter',?3,'',?4)",
            rusqlite::params![new_id(), format!("{start}~{end}"), amounts, now_iso()],
        )?;
        crate::audit::log_change(c, "filing_draft", "", "insert", &format!("生成底稿 {start}~{end}"));
        Ok(html)
    })?;
    export_document(format!("季报底稿-{}-{}", start, end), html)
}

// ── 工单与协作人（T24）───────────────────────────────────

#[derive(Serialize)]
pub struct PartyRow {
    pub id: String,
    pub name: String,
    pub ptype: String,
    pub id_card: Option<String>,
    pub contact: Option<String>,
    pub total_paid: Option<String>,
}

#[derive(Serialize, Deserialize)]
pub struct PartyInput {
    pub name: String,
    pub ptype: String,
    pub id_card: Option<String>,
    pub contact: Option<String>,
}

#[tauri::command]
pub fn list_parties(state: tauri::State<'_, DbState>) -> Result<Vec<PartyRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT p.id,p.name,p.type,p.id_card,p.contact,
                    (SELECT COALESCE(SUM(CAST(e.amount AS REAL)),0) FROM event e WHERE e.party_id=p.id AND e.direction IN ('expense'))
             FROM party p WHERE p.type='person' ORDER BY p.created_at DESC LIMIT 200",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let total: f64 = r.get(5)?;
                Ok(PartyRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    ptype: r.get(2)?,
                    id_card: r.get(3)?,
                    contact: r.get(4)?,
                    total_paid: Some(format!("{total:.2}")),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn add_party(state: tauri::State<'_, DbState>, input: PartyInput) -> Result<String, String> {
    let id = new_id();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO party(id,name,name_norm,type,id_card,contact,created_at) VALUES(?1,?2,?2,?3,?4,?5,?6)",
            rusqlite::params![id, input.name, input.ptype, input.id_card, input.contact, now_iso()],
        )?;
        Ok(id)
    })
}

#[derive(Serialize)]
pub struct JobRow {
    pub id: String,
    pub title: String,
    pub worker: Option<String>,
    pub start_on: Option<String>,
    pub end_on: Option<String>,
    pub total_agreed: Option<String>,
    pub total_paid: String,
    pub status: String,
}

#[derive(Serialize, Deserialize)]
pub struct JobInput {
    pub title: String,
    pub worker_id: Option<String>,
    pub start_on: Option<String>,
    pub end_on: Option<String>,
    pub total_agreed: Option<String>,
}

#[tauri::command]
pub fn list_jobs(state: tauri::State<'_, DbState>) -> Result<Vec<JobRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT j.id,j.title,p.name,j.start_on,j.end_on,j.total_agreed,j.total_paid,j.status
             FROM service_job j LEFT JOIN party p ON p.id=j.worker_id
             ORDER BY j.created_at DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(JobRow {
                    id: r.get(0)?,
                    title: r.get(1)?,
                    worker: r.get(2)?,
                    start_on: r.get(3)?,
                    end_on: r.get(4)?,
                    total_agreed: r.get(5)?,
                    total_paid: r.get(6)?,
                    status: r.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn add_job(state: tauri::State<'_, DbState>, input: JobInput) -> Result<String, String> {
    let id = new_id();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO service_job(id,title,worker_id,start_on,end_on,total_agreed,total_paid,status,created_at)
             VALUES(?1,?2,?3,?4,?5,?6,'0','open',?7)",
            rusqlite::params![id, input.title, input.worker_id, input.start_on, input.end_on, input.total_agreed, now_iso()],
        )?;
        Ok(id)
    })
}

/// 工单付款（分期支付合并判定：累加 total_paid，超 500 提示由前端展示）
#[tauri::command]
pub fn job_pay(state: tauri::State<'_, DbState>, job_id: String, amount: String) -> Result<String, String> {
    let now = now_iso();
    with_db(state.inner(), |c| {
        c.execute(
            "UPDATE service_job SET total_paid = CAST(COALESCE(total_paid,'0') AS REAL) + CAST(?1 AS REAL) WHERE id=?2",
            rusqlite::params![amount, job_id],
        )?;
        let job: (Option<String>, Option<String>) = c
            .query_row("SELECT worker_id, title FROM service_job WHERE id=?1", [&job_id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        let acct_id: String = match c.query_row("SELECT id FROM account WHERE name='公户'", [], |r| r.get(0)) {
            Ok(a) => a,
            Err(_) => {
                let a = new_id();
                c.execute(
                    "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,'公户','bank',1,'0',?2)",
                    rusqlite::params![a, now],
                )?;
                a
            }
        };
        let total_paid: f64 = c.query_row(
            "SELECT CAST(COALESCE(total_paid,'0') AS REAL) FROM service_job WHERE id=?1",
            [&job_id],
            |r| r.get(0),
        )?;
        // 超 500 且未开票 → 待补票；≤500 → 收据可扣（按事项总额判定，PRD §5.13）
        let tax_status = if total_paid <= 500.0 { "收据可扣" } else { "待补票" };
        let eid = new_id();
        c.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,job_id,created_at,updated_at)
             VALUES(?1,?2,'expense',?3,?4,?5,'none',?6,'劳务人工费','manual',?7,'none','none',?8,?9,?9)",
            rusqlite::params![eid, now.chars().take(10).collect::<String>(), amount, job.0, acct_id, job.1, tax_status, job_id, now],
        )?;
        Ok(eid)
    })
}

#[tauri::command]
pub fn job_settle(state: tauri::State<'_, DbState>, job_id: String) -> Result<(), String> {
    with_db(state.inner(), |c| {
        c.execute("UPDATE service_job SET status='settled' WHERE id=?1", [&job_id])?;
        Ok(())
    })
}

/// 记劳务预扣（个税台账，payee+period 单点落账）
#[tauri::command]
pub fn add_withhold(state: tauri::State<'_, DbState>, payee_id: String, income: String, period: String) -> Result<String, String> {
    let withheld = crate::tax::labor_withhold(income.parse().unwrap_or(0.0));
    let id = new_id();
    let now = now_iso();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO withhold_ledger(id,paid_on,payee_id,kind,income,withheld,period)
             VALUES(?1,?2,?3,'labor',?4,?5,?6)
             ON CONFLICT DO UPDATE SET income=?4, withheld=?5",
            rusqlite::params![id, now.chars().take(10).collect::<String>(), payee_id, income, format!("{withheld:.2}"), period],
        )?;
        Ok(id)
    })
}

// ── 正式员工 / 工资绩效（v1.5，报销联动）──────────────────

#[derive(Serialize)]
pub struct EmployeeRow {
    pub id: String,
    pub name: String,
    pub id_card: Option<String>,
    pub title: Option<String>,
    pub hire_on: Option<String>,
    pub base_salary: Option<String>,
    pub rb_count: i64,
    pub rb_total: String,
    pub advance_open: String, // 未清垫付
}

#[derive(Serialize, Deserialize)]
pub struct EmployeeInput {
    pub name: String,
    pub id_card: Option<String>,
    pub title: Option<String>,
    pub hire_on: Option<String>,
    pub base_salary: Option<String>,
}

#[tauri::command]
pub fn list_employees(state: tauri::State<'_, DbState>) -> Result<Vec<EmployeeRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT p.id,p.name,p.id_card,p.title,p.hire_on,p.base_salary,
                    (SELECT COUNT(*) FROM reimbursement r WHERE r.party_id=p.id OR r.applicant=p.name),
                    (SELECT COALESCE(SUM(CAST(r.total AS REAL)),0) FROM reimbursement r WHERE r.party_id=p.id OR r.applicant=p.name),
                    (SELECT COALESCE(SUM(CAST(e.amount AS REAL)),0) FROM event e
                      WHERE e.party_id=p.id AND e.direction='expense' AND e.reimburse_status='none'
                        AND e.source<>'payroll')
             FROM party p WHERE p.type='person' AND p.title IS NOT NULL
             ORDER BY p.created_at DESC LIMIT 100",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let rb_total: f64 = r.get(7)?;
                let adv: f64 = r.get(8)?;
                Ok(EmployeeRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    id_card: r.get(2)?,
                    title: r.get(3)?,
                    hire_on: r.get(4)?,
                    base_salary: r.get(5)?,
                    rb_count: r.get(6)?,
                    rb_total: format!("{rb_total:.2}"),
                    advance_open: format!("{adv:.2}"),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn add_employee(state: tauri::State<'_, DbState>, input: EmployeeInput) -> Result<String, String> {
    let id = new_id();
    with_db(state.inner(), |c| {
        // 身份证唯一：已有同身份证的 person 则升级为员工
        if let Some(idc) = input.id_card.clone() {
            if !idc.is_empty() {
                let existing: Option<String> = c
                    .query_row("SELECT id FROM party WHERE id_card=?1", [&idc], |r| r.get(0))
                    .ok();
                if let Some(eid) = existing {
                    c.execute(
                        "UPDATE party SET title=?1, hire_on=?2, base_salary=?3 WHERE id=?4",
                        rusqlite::params![input.title, input.hire_on, input.base_salary, eid],
                    )?;
                    return Ok(eid);
                }
            }
        }
        c.execute(
            "INSERT INTO party(id,name,name_norm,type,id_card,contact,title,hire_on,base_salary,created_at)
             VALUES(?1,?2,?2,'person',?3,NULL,?4,?5,?6,?7)",
            rusqlite::params![id, input.name, input.id_card, input.title, input.hire_on, input.base_salary, now_iso()],
        )?;
        Ok(id)
    })
}

#[derive(Serialize)]
pub struct PayrollRow {
    pub id: String,
    pub period_month: String,
    pub employee: String,
    pub gross: String,
    pub si_hf: String,
    pub tax: String,
    pub net: String,
}

/// 工资个税预览（累计预扣法）
#[tauri::command]
pub fn wage_tax_preview(
    state: tauri::State<'_, DbState>,
    employee_id: String,
    period_month: String,
    gross: String,
    si_hf: String,
) -> Result<f64, String> {
    let g: f64 = gross.parse().unwrap_or(0.0);
    let si: f64 = si_hf.parse().unwrap_or(0.0);
    with_db(state.inner(), |c| {
        let year = period_month.chars().take(4).collect::<String>();
        let (cg, cs, ct, n): (f64, f64, f64, i64) = {
            let row = c.query_row(
                "SELECT COALESCE(SUM(CAST(gross AS REAL)),0), COALESCE(SUM(CAST(si_hf AS REAL)),0),
                        COALESCE(SUM(CAST(tax AS REAL)),0), COUNT(*)
                 FROM payroll WHERE employee_id=?1 AND substr(period_month,1,4)=?2",
                rusqlite::params![employee_id, year],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?;
            row
        };
        let months = (n as u32) + 1;
        Ok(crate::tax::wage_withhold_cumulative(cg, cs, ct, months, g, si))
    })
}

/// 录入工资单：写 payroll + 生成实发工资支出事项 + 个税台账（kind=wage）
#[tauri::command]
pub fn add_payroll(
    state: tauri::State<'_, DbState>,
    employee_id: String,
    period_month: String,
    gross: String,
    si_hf: String,
    tax: String,
    net: String,
) -> Result<String, String> {
    let id = new_id();
    let now = now_iso();
    let today: String = now.chars().take(10).collect();
    let mut guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let c = guard.as_mut().ok_or("db not initialized")?;
    let dup: i64 = c
        .query_row(
            "SELECT COUNT(*) FROM payroll WHERE employee_id=?1 AND period_month=?2",
            rusqlite::params![employee_id, period_month],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    if dup > 0 {
        return Err("该员工该月已有工资单".to_string());
    }
    let name: String = c
        .query_row("SELECT name FROM party WHERE id=?1", [&employee_id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO payroll(id,period_month,employee_id,gross,si_hf,tax,net,dual_salary_warn,created_at)
         VALUES(?1,?2,?3,?4,?5,?6,?7,0,?8)",
        rusqlite::params![id, period_month, employee_id, gross, si_hf, tax, net, now],
    )
    .map_err(|e| e.to_string())?;
    let acct_id: String = match c.query_row("SELECT id FROM account WHERE name='公户'", [], |r| r.get(0)) {
        Ok(a) => a,
        Err(_) => {
            let a = new_id();
            c.execute(
                "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,'公户','bank',1,'0',?2)",
                rusqlite::params![a, now],
            )
            .map_err(|e| e.to_string())?;
            a
        }
    };
    c.execute(
        "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
         VALUES(?1,?2,'expense',?3,?4,?5,'none',?6,'工资薪金','payroll','可扣','none','none',?7,?7)",
        rusqlite::params![new_id(), today, net, name, acct_id, format!("工资 {period_month}"), now],
    )
    .map_err(|e| e.to_string())?;
    c.execute(
        "INSERT INTO withhold_ledger(id,paid_on,payee_id,kind,income,withheld,period)
         VALUES(?1,?2,?3,'wage',?4,?5,?6)",
        rusqlite::params![new_id(), today, employee_id, gross, tax, period_month],
    )
    .map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn list_payroll(state: tauri::State<'_, DbState>) -> Result<Vec<PayrollRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT p.id,p.period_month,pa.name,p.gross,p.si_hf,p.tax,p.net
             FROM payroll p JOIN party pa ON pa.id=p.employee_id
             ORDER BY p.period_month DESC LIMIT 60",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(PayrollRow {
                    id: r.get(0)?,
                    period_month: r.get(1)?,
                    employee: r.get(2)?,
                    gross: r.get(3)?,
                    si_hf: r.get(4)?,
                    tax: r.get(5)?,
                    net: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

/// 绩效/奖金：直接生成支出事项（可后续并入工资单）
#[tauri::command]
pub fn add_bonus(
    state: tauri::State<'_, DbState>,
    employee_id: String,
    amount: String,
    memo: String,
) -> Result<String, String> {
    let now = now_iso();
    let today: String = now.chars().take(10).collect();
    with_db(state.inner(), |c| {
        let name: String = c.query_row("SELECT name FROM party WHERE id=?1", [&employee_id], |r| r.get(0))?;
        let acct_id: String = match c.query_row("SELECT id FROM account WHERE name='公户'", [], |r| r.get(0)) {
            Ok(a) => a,
            Err(_) => {
                let a = new_id();
                c.execute(
                    "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,'公户','bank',1,'0',?2)",
                    rusqlite::params![a, now],
                )?;
                a
            }
        };
        let eid = new_id();
        c.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES(?1,?2,'expense',?3,?4,?5,'none',?6,'绩效奖金','payroll','可扣','none','none',?7,?7)",
            rusqlite::params![eid, today, amount, name, acct_id, memo, now],
        )?;
        Ok(eid)
    })
}

// ── 账户模块（v1.6）：总览/余额/流水/转账 ──────────────────

#[derive(Serialize)]
pub struct AccountRow {
    pub id: String,
    pub name: String,
    pub atype: String,
    pub is_company: bool,
    pub opening_balance: String,
    pub opening_date: Option<String>,
    pub balance: f64,
    pub event_count: i64,
}

#[tauri::command]
pub fn list_accounts(state: tauri::State<'_, DbState>) -> Result<Vec<AccountRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT a.id,a.name,a.type,a.is_company,a.opening_balance,a.opening_date,
                    (SELECT COALESCE(SUM(CASE WHEN e.direction='income' THEN CAST(e.amount AS REAL)
                                              WHEN e.direction='expense' THEN -CAST(e.amount AS REAL)
                                              WHEN e.direction='transfer' THEN 0 ELSE 0 END),0)
                       FROM event e WHERE e.account_id=a.id),
                    (SELECT COUNT(*) FROM event e WHERE e.account_id=a.id)
             FROM account a ORDER BY a.is_company DESC, a.created_at LIMIT 50",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let opening: String = r.get(4)?;
                let flow: f64 = r.get(6)?;
                let bal = opening.parse::<f64>().unwrap_or(0.0) + flow;
                Ok(AccountRow {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    atype: r.get(2)?,
                    is_company: r.get::<_, i64>(3)? == 1,
                    opening_balance: opening,
                    opening_date: r.get(5)?,
                    balance: (bal * 100.0).round() / 100.0,
                    event_count: r.get(7)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[derive(Serialize, Deserialize)]
pub struct AccountInput {
    pub name: String,
    pub atype: Option<String>,
    pub opening_balance: Option<String>,
    pub opening_date: Option<String>,
    pub is_company: Option<bool>,
}

#[tauri::command]
pub fn add_account(state: tauri::State<'_, DbState>, input: AccountInput) -> Result<String, String> {
    let id = new_id();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO account(id,name,type,is_company,opening_balance,opening_date,created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7)",
            rusqlite::params![
                id,
                input.name,
                input.atype.unwrap_or_else(|| "bank".into()),
                if input.is_company.unwrap_or(true) { 1 } else { 0 },
                input.opening_balance.unwrap_or_else(|| "0".into()),
                input.opening_date,
                now_iso()
            ],
        )?;
        Ok(id)
    })
}

#[tauri::command]
pub fn account_events(state: tauri::State<'_, DbState>, account_id: String, limit: i64) -> Result<Vec<EventRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,occurred_on,direction,amount,counterparty_raw,memo,category,tax_status,source
             FROM event WHERE account_id=?1 ORDER BY occurred_on DESC, created_at DESC LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![account_id, limit], |r| {
                Ok(EventRow {
                    id: r.get(0)?,
                    occurred_on: r.get(1)?,
                    direction: r.get(2)?,
                    amount: r.get(3)?,
                    counterparty: r.get(4)?,
                    memo: r.get(5)?,
                    category: r.get(6)?,
                    tax_status: r.get(7)?,
                    source: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

/// 账户间转账：生成配对 transfer 事项（pair_id 关联，不计入收支）
#[tauri::command]
pub fn transfer(
    state: tauri::State<'_, DbState>,
    from_id: String,
    to_id: String,
    amount: String,
    occurred_on: String,
    memo: Option<String>,
) -> Result<String, String> {
    if from_id == to_id {
        return Err("转出与转入账户不能相同".into());
    }
    let now = now_iso();
    let pair = new_id();
    with_db(state.inner(), |c| {
        let from_name: String = c.query_row("SELECT name FROM account WHERE id=?1", [&from_id], |r| r.get(0))?;
        let to_name: String = c.query_row("SELECT name FROM account WHERE id=?1", [&to_id], |r| r.get(0))?;
        let m = memo.unwrap_or_else(|| format!("{from_name} → {to_name}"));
        let e1 = new_id();
        let e2 = new_id();
        c.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES(?1,?2,'transfer',?3,?4,?5,?6,'paired',?7,'账户调拨','manual','不涉税','none','none',?8,?8)",
            rusqlite::params![e1, occurred_on, amount, to_name, from_id, pair, m, now],
        )?;
        c.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES(?1,?2,'transfer',?3,?4,?5,?6,'paired',?7,'账户调拨','manual','不涉税','none','none',?8,?8)",
            rusqlite::params![e2, occurred_on, amount, from_name, to_id, pair, m, now],
        )?;
        Ok(pair)
    })
}

/// 编辑账户
#[tauri::command]
pub fn update_account(state: tauri::State<'_, DbState>, input: AccountInput, id: String) -> Result<(), String> {
    with_db(state.inner(), |c| {
        c.execute(
            "UPDATE account SET name=?1, type=?2, is_company=?3, opening_balance=?4, opening_date=?5 WHERE id=?6",
            rusqlite::params![
                input.name,
                input.atype.unwrap_or_else(|| "bank".into()),
                if input.is_company.unwrap_or(true) { 1 } else { 0 },
                input.opening_balance.unwrap_or_else(|| "0".into()),
                input.opening_date,
                id
            ],
        )?;
        Ok(())
    })
}

/// 删除账户（有流水则拒绝，防误删账务数据）
#[tauri::command]
pub fn delete_account(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let c = guard.as_ref().ok_or("db not initialized")?;
    let n: i64 = c
        .query_row("SELECT COUNT(*) FROM event WHERE account_id=?1", [&id], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if n > 0 {
        return Err(format!("该账户有 {n} 笔流水，不能删除——可改名或先清理事项"));
    }
    c.execute("DELETE FROM account WHERE id=?1", [&id]).map_err(|e| e.to_string())?;
    Ok(())
}

/// 账户转账记录（该账户名下调拨事项）
#[tauri::command]
pub fn account_transfers(state: tauri::State<'_, DbState>, account_id: String) -> Result<Vec<EventRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,occurred_on,direction,amount,counterparty_raw,memo,category,tax_status,source
             FROM event WHERE account_id=?1 AND direction='transfer' ORDER BY occurred_on DESC LIMIT 50",
        )?;
        let rows = stmt
            .query_map([account_id], |r| {
                Ok(EventRow {
                    id: r.get(0)?,
                    occurred_on: r.get(1)?,
                    direction: r.get(2)?,
                    amount: r.get(3)?,
                    counterparty: r.get(4)?,
                    memo: r.get(5)?,
                    category: r.get(6)?,
                    tax_status: r.get(7)?,
                    source: r.get(8)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

/// 通用文本导出（CSV 等）：写 data/exports 并返回路径
#[tauri::command]
pub fn export_text(name: String, content: String, ext: String) -> Result<String, String> {
    let dir = crate::db::data_dir().join("exports");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let safe: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    let e = ext.trim_start_matches('.').to_ascii_lowercase();
    let out = dir.join(format!("{safe}.{e}"));
    std::fs::write(&out, content).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

// ── 流水编辑/删除（v1.8） ─────────────────────────────────

#[derive(Serialize, Deserialize)]
pub struct EventPatch {
    pub occurred_on: Option<String>,
    pub direction: Option<String>,
    pub amount: Option<String>,
    pub counterparty: Option<String>,
    pub memo: Option<String>,
    pub category: Option<String>,
    pub tax_status: Option<String>,
    pub account_name: Option<String>,
}

#[tauri::command]
pub fn update_event(state: tauri::State<'_, DbState>, id: String, patch: EventPatch) -> Result<(), String> {
    with_db(state.inner(), |c| {
        if let Some(v) = patch.occurred_on {
            c.execute("UPDATE event SET occurred_on=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.direction {
            c.execute("UPDATE event SET direction=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.amount {
            c.execute("UPDATE event SET amount=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.counterparty {
            c.execute("UPDATE event SET counterparty_raw=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.memo {
            c.execute("UPDATE event SET memo=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.category {
            c.execute("UPDATE event SET category=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.tax_status {
            c.execute("UPDATE event SET tax_status=?1 WHERE id=?2", rusqlite::params![v, id])?;
        }
        if let Some(v) = patch.account_name {
            let acct_id: Option<String> = c
                .query_row("SELECT id FROM account WHERE name=?1", [&v], |r| r.get(0))
                .ok();
            if let Some(a) = acct_id {
                c.execute("UPDATE event SET account_id=?1 WHERE id=?2", rusqlite::params![a, id])?;
            }
        }
        c.execute("UPDATE event SET updated_at=?1 WHERE id=?2", rusqlite::params![now_iso(), id])?;
        Ok(())
    })
}

#[tauri::command]
pub fn delete_event(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    with_db(state.inner(), |c| {
        c.execute("DELETE FROM event WHERE id=?1", [&id])?;
        crate::audit::log_change(c, "event", &id, "delete", "删除流水");
        Ok(())
    })
}

// ── 规则沉淀（T25）与经营助手（T26）─────────────────────

#[tauri::command]
pub fn list_rules(state: tauri::State<'_, DbState>) -> Result<Vec<crate::rules::RuleRow>, String> {
    with_db(state.inner(), |c| crate::rules::list_rules(c))
}

#[tauri::command]
pub fn toggle_rule(state: tauri::State<'_, DbState>, id: String) -> Result<(), String> {
    with_db(state.inner(), |c| crate::rules::toggle_rule(c, &id))
}

#[tauri::command]
pub fn apply_rules(state: tauri::State<'_, DbState>) -> Result<u64, String> {
    with_db(state.inner(), |c| crate::rules::apply_rules(c))
}

#[tauri::command]
pub fn ar_ap(state: tauri::State<'_, DbState>) -> Result<Vec<crate::ops::ArApRow>, String> {
    with_db(state.inner(), |c| crate::ops::ar_ap(c))
}

#[tauri::command]
pub fn settle_event(state: tauri::State<'_, DbState>, event_id: String) -> Result<(), String> {
    with_db(state.inner(), |c| crate::ops::settle_event(c, &event_id))
}

#[tauri::command]
pub fn biz_brief(state: tauri::State<'_, DbState>) -> Result<crate::ops::BizBrief, String> {
    with_db(state.inner(), |c| crate::ops::brief(c))
}

// ── 邮箱收票 / 外部报销单 / AI 总监 ────────────────────────

#[tauri::command]
pub fn fetch_invoices(
    state: tauri::State<'_, DbState>,
    host: String,
    port: u16,
    user: String,
    pass: String,
    folder: Option<String>,
) -> Result<crate::mail::FetchReport, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::mail::fetch_invoices(conn, &host, port, &user, &pass, &folder.unwrap_or_else(|| "INBOX".into()))
}

#[tauri::command]
pub fn parse_external_doc(
    state: tauri::State<'_, DbState>,
    path: String,
    applicant: Option<String>,
) -> Result<crate::import_doc::DocParseResult, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::import_doc::parse_external_doc(conn, &path, applicant)
}

#[tauri::command]
pub fn ai_chat(
    state: tauri::State<'_, DbState>,
    req: crate::assistant::ChatRequest,
) -> Result<crate::assistant::ChatReply, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::assistant::chat(conn, req)
}

#[tauri::command]
pub fn ai_rule_pack() -> Result<String, String> {
    Ok(crate::assistant::RULE_PACK.to_string())
}

/// 加密备份（AES-256-GCM，口令派生密钥）
#[tauri::command]
pub fn backup_encrypted(password: String) -> Result<String, String> {
    let path = crate::db::resolve_db_path();
    crate::crypto::backup_encrypted(&path.display().to_string(), &password)
}

#[tauri::command]
pub fn verify_encrypted(path: String, password: String) -> Result<bool, String> {
    crate::crypto::verify_encrypted(&path, &password)
}

// ── 统计序列 / 申报表格 / 任务种子（重构批次）─────────────

#[tauri::command]
pub fn stats_series(state: tauri::State<'_, DbState>) -> Result<crate::report::StatsSeries, String> {
    with_db(state.inner(), |c| crate::report::stats_series(c))
}

#[tauri::command]
pub fn seed_default_tasks(state: tauri::State<'_, DbState>) -> Result<u64, String> {
    with_db(state.inner(), |c| crate::report::seed_default_tasks(c))
}

#[tauri::command]
pub fn export_filing_forms(state: tauri::State<'_, DbState>, start: String, end: String) -> Result<String, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    let html = crate::report::render_filing_forms(conn, &start, &end)?;
    export_document(format!("税务申报表格-{start}-{end}"), html)
}

// ── 全局搜索（发票/凭证/流水）────────────────────────────

#[derive(Serialize)]
pub struct SearchHit {
    pub kind: String, // event | voucher
    pub id: String,
    pub title: String,
    pub sub: String,
    pub amount: String,
}

#[tauri::command]
pub fn search_all(state: tauri::State<'_, DbState>, q: String) -> Result<Vec<SearchHit>, String> {
    let like = format!("%{q}%");
    with_db(state.inner(), |c| {
        let mut out = Vec::new();
        let mut stmt = c.prepare(
            "SELECT id,occurred_on,direction,amount,counterparty_raw,memo FROM event
             WHERE counterparty_raw LIKE ?1 OR memo LIKE ?1 OR amount LIKE ?1 OR occurred_on LIKE ?1
             ORDER BY occurred_on DESC LIMIT 8",
        )?;
        let rows = stmt
            .query_map([&like], |r| {
                Ok(SearchHit {
                    kind: "event".into(),
                    id: r.get(0)?,
                    title: format!("{} · ¥{}", r.get::<_, String>(1)?, r.get::<_, String>(3)?),
                    sub: format!("{} {}", r.get::<_, Option<String>>(4)?.unwrap_or_default(), r.get::<_, Option<String>>(5)?.unwrap_or_default()),
                    amount: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        out.extend(rows);

        let mut stmt2 = c.prepare(
            "SELECT id,invoice_no,seller,amount,issued_on FROM voucher
             WHERE invoice_no LIKE ?1 OR seller LIKE ?1 OR buyer LIKE ?1 OR amount LIKE ?1
             ORDER BY issued_on DESC LIMIT 8",
        )?;
        let rows2 = stmt2
            .query_map([&like], |r| {
                Ok(SearchHit {
                    kind: "voucher".into(),
                    id: r.get(0)?,
                    title: format!("票 {} · ¥{}", r.get::<_, Option<String>>(1)?.unwrap_or_default(), r.get::<_, Option<String>>(3)?.unwrap_or_default()),
                    sub: format!("{} · {}", r.get::<_, Option<String>>(2)?.unwrap_or_default(), r.get::<_, Option<String>>(4)?.unwrap_or_default()),
                    amount: r.get::<_, Option<String>>(3)?.unwrap_or_default(),
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        out.extend(rows2);
        Ok(out)
    })
}

fn normalize_date(s: &str) -> String {
    let d: String = s.chars().filter(|c| c.is_ascii_digit()).collect();
    if d.len() == 8 {
        return format!("{}-{}-{}", &d[0..4], &d[4..6], &d[6..8]);
    }
    s.chars()
        .take(10)
        .collect::<String>()
        .replace('/', "-")
}

#[tauri::command]
pub fn import_bank_csv(state: tauri::State<'_, DbState>, path: String) -> Result<u64, String> {
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(text.as_bytes());
    let file_hash = format!("{:x}", hasher.finalize());
    {
        let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("db not initialized")?;
        if crate::audit::batch_seen(conn, &file_hash) {
            return Ok(0); // 同一文件已导入过
        }
    }
    let mut lines = text.lines().filter(|l| !l.trim().is_empty());
    let first = lines.next().unwrap_or("").to_string();
    let header: Vec<String> = first.split(',').map(|s| s.trim().to_string()).collect();
    let find = |kws: &[&str]| -> Option<usize> {
        header.iter().position(|h| kws.iter().any(|k| h.contains(k)))
    };
    let (c_date, c_cp, c_memo, c_inc, c_exp) = (
        find(&["日期"]),
        find(&["对方", "对手", "户名", "收款人"]),
        find(&["摘要", "用途", "事由", "备注"]),
        find(&["收入", "存入"]),
        find(&["支出", "取出"]),
    );
    let c_date = c_date.ok_or("未识别日期列")?;
    let mut count = 0u64;
    let now = now_iso();
    let mut guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    let acct_id: String = match conn.query_row("SELECT id FROM account WHERE name='公户'", [], |r| r.get(0)) {
        Ok(a) => a,
        Err(_) => {
            let a = new_id();
            conn.execute(
                "INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES(?1,'公户','bank',1,'0',?2)",
                rusqlite::params![a, now],
            )
            .map_err(|e| e.to_string())?;
            a
        }
    };
    for line in lines {
        let cells: Vec<&str> = line.split(',').collect();
        let get = |i: Option<usize>| -> String {
            i.map(|i| cells.get(i).copied().unwrap_or("").trim().to_string())
                .unwrap_or_default()
        };
        let date = normalize_date(&get(Some(c_date)));
        if date.len() < 8 {
            continue;
        }
        let inc = parse_f(&get(c_inc)).unwrap_or(0.0);
        let exp = parse_f(&get(c_exp)).unwrap_or(0.0);
        let (direction, amount) = if inc > 0.0 {
            ("income", inc)
        } else if exp > 0.0 {
            ("expense", exp)
        } else {
            continue;
        };
        conn.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,'none',?7,'bank_csv',NULL,'none','none',?8,?8)",
            rusqlite::params![
                new_id(),
                date,
                direction,
                format!("{amount:.2}"),
                get(c_cp),
                acct_id,
                get(c_memo),
                now
            ],
        )
        .map_err(|e| e.to_string())?;
        count += 1;
    }
    crate::audit::log_batch(conn, "bank_csv", &path, &file_hash, count as i64);
    crate::audit::log_change(conn, "event", "", "insert", &format!("导入流水 {count} 笔"));
    Ok(count)
}

// ── 票夹收票：本地附件导入 / 文件选择 / 邮箱绑定 ─────────────

#[tauri::command]
pub fn import_voucher_files(
    state: tauri::State<'_, DbState>,
    paths: Vec<String>,
) -> Result<Vec<crate::intake::ImportResult>, String> {
    crate::intake::with_conn(state.inner(), |conn| Ok(crate::intake::import_files(conn, &paths)))
}

#[tauri::command]
pub fn pick_voucher_files() -> Vec<String> {
    crate::intake::pick_files()
}

#[tauri::command]
pub fn mail_config_get(
    state: tauri::State<'_, DbState>,
) -> Result<Option<crate::intake::MailConfigView>, String> {
    crate::intake::with_conn(state.inner(), |conn| Ok(crate::intake::config_get(conn)))
}

#[tauri::command]
pub fn mail_config_save(
    state: tauri::State<'_, DbState>,
    host: String,
    port: u16,
    user: String,
    pass: Option<String>,
    folder: Option<String>,
    auto: bool,
) -> Result<(), String> {
    crate::intake::with_conn(state.inner(), |conn| {
        crate::intake::config_save(conn, host, port, user, pass, folder, auto)
    })
}

#[tauri::command]
pub fn mail_config_clear(state: tauri::State<'_, DbState>) -> Result<(), String> {
    crate::intake::with_conn(state.inner(), |conn| crate::intake::config_clear(conn))
}

#[tauri::command]
pub fn fetch_invoices_saved(
    state: tauri::State<'_, DbState>,
    force: Option<bool>,
) -> Result<crate::mail::FetchReport, String> {
    crate::intake::with_conn(state.inner(), |conn| {
        crate::intake::fetch_with_saved(conn, force.unwrap_or(false)).map(|(r, _)| r)
    })
}

#[tauri::command]
pub fn open_voucher_file(
    state: tauri::State<'_, DbState>,
    id: String,
) -> Result<String, String> {
    crate::intake::with_conn(state.inner(), |conn| {
        crate::intake::open_attachment(conn, &id)
    })
}

// ── 备份恢复 / 更新源 / 许可 / 工时 / 申报底稿历史 ────────────

fn reopen_db(state: &DbState) -> Result<(), String> {
    let path = crate::db::resolve_db_path();
    let conn = crate::cryptdb::open_app_db(&path)?;
    let mut guard = state.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(conn);
    Ok(())
}

#[tauri::command]
pub fn backups_list() -> Vec<crate::maintenance::BackupItem> {
    crate::maintenance::list_backups()
}

#[tauri::command]
pub fn restore_backup(state: tauri::State<'_, DbState>, path: String) -> Result<String, String> {
    let db_path = crate::db::resolve_db_path();
    let src = std::path::PathBuf::from(&path);
    if !src.exists() {
        return Err("备份文件不存在".into());
    }
    // 恢复前先给当前库做快照，失败可回滚
    let snap = {
        let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("db not initialized")?;
        crate::maintenance::snapshot(conn)?
    };
    {
        let mut guard = state.inner().0.lock().map_err(|e| e.to_string())?;
        let _ = guard.take();
    }

    let plain = crate::cryptdb::is_plaintext(&src);
    let rollback = || -> Result<(), String> {
        let _ = std::fs::copy(&snap, &db_path);
        reopen_db(state.inner())
    };

    // 清掉 WAL/SHM，避免旧日志污染新库
    for suf in ["-wal", "-shm"] {
        let mut side = db_path.as_os_str().to_os_string();
        side.push(suf);
        let _ = std::fs::remove_file(std::path::PathBuf::from(side));
    }
    if let Err(e) = std::fs::copy(&src, &db_path) {
        let _ = rollback();
        return Err(format!("替换数据库失败：{e}"));
    }
    if plain {
        let key = crate::cryptdb::local_key()?;
        if let Err(e) = crate::cryptdb::import_plaintext(&db_path, &key) {
            let _ = rollback();
            return Err(e);
        }
    }
    match reopen_db(state.inner()) {
        Ok(_) => {
            if let Ok(guard) = state.inner().0.lock() {
                if let Some(c) = guard.as_ref() {
                    crate::audit::log_change(c, "db", "", "update", &format!("从备份恢复：{path}"));
                }
            }
            Ok(snap)
        }
        Err(e) => {
            let _ = rollback();
            Err(format!("恢复失败，已回滚到恢复前状态：{e}"))
        }
    }
}

#[tauri::command]
pub fn restore_encrypted(
    state: tauri::State<'_, DbState>,
    path: String,
    password: String,
) -> Result<String, String> {
    let blob = std::fs::read(&path).map_err(|e| format!("读取备份失败：{e}"))?;
    let plain = crate::crypto::decrypt_blob(&blob, &password)?;
    let tmp = crate::maintenance::backups_dir().join(format!("restore-{}.db", now_iso()));
    std::fs::write(&tmp, &plain).map_err(|e| format!("写入临时库失败：{e}"))?;
    let out = restore_backup_inner(state.inner(), &tmp);
    let _ = std::fs::remove_file(&tmp);
    out
}

/// 与 restore_backup 同样的逻辑，但接收已解压的明文库路径（供加密恢复复用）
fn restore_backup_inner(state: &DbState, src: &std::path::Path) -> Result<String, String> {
    let db_path = crate::db::resolve_db_path();
    let snap = {
        let guard = state.0.lock().map_err(|e| e.to_string())?;
        let conn = guard.as_ref().ok_or("db not initialized")?;
        crate::maintenance::snapshot(conn)?
    };
    {
        let mut guard = state.0.lock().map_err(|e| e.to_string())?;
        let _ = guard.take();
    }
    let rollback = || -> Result<(), String> {
        let _ = std::fs::copy(&snap, &db_path);
        reopen_db(state)
    };
    for suf in ["-wal", "-shm"] {
        let mut side = db_path.as_os_str().to_os_string();
        side.push(suf);
        let _ = std::fs::remove_file(std::path::PathBuf::from(side));
    }
    if let Err(e) = std::fs::copy(src, &db_path) {
        let _ = rollback();
        return Err(format!("替换数据库失败：{e}"));
    }
    if crate::cryptdb::is_plaintext(&db_path) {
        let key = crate::cryptdb::local_key()?;
        if let Err(e) = crate::cryptdb::import_plaintext(&db_path, &key) {
            let _ = rollback();
            return Err(e);
        }
    }
    match reopen_db(state) {
        Ok(_) => {
            if let Ok(guard) = state.0.lock() {
                if let Some(c) = guard.as_ref() {
                    crate::audit::log_change(c, "db", "", "update", "从加密备份恢复");
                }
            }
            Ok(snap)
        }
        Err(e) => {
            let _ = rollback();
            Err(format!("恢复失败，已回滚：{e}"))
        }
    }
}

#[tauri::command]
pub fn update_source_get(state: tauri::State<'_, DbState>) -> Result<String, String> {
    with_db(state.inner(), |c| Ok(crate::maintenance::update_source_get(c)))
}

#[tauri::command]
pub fn update_source_save(state: tauri::State<'_, DbState>, url: String) -> Result<(), String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::maintenance::update_source_save(conn, &url)
}

#[tauri::command]
pub fn update_check(state: tauri::State<'_, DbState>) -> Result<crate::maintenance::UpdateCheck, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::maintenance::update_check(conn)
}

#[tauri::command]
pub fn license_info(state: tauri::State<'_, DbState>) -> Result<crate::maintenance::LicenseInfo, String> {
    with_db(state.inner(), |c| Ok(crate::maintenance::license_info(c)))
}

#[derive(Serialize)]
pub struct WorklogRow {
    pub id: String,
    pub job_id: String,
    pub work_on: String,
    pub hours: Option<String>,
    pub note: Option<String>,
}

#[tauri::command]
pub fn add_worklog(
    state: tauri::State<'_, DbState>,
    job_id: String,
    work_on: String,
    hours: Option<String>,
    note: Option<String>,
) -> Result<String, String> {
    let id = new_id();
    with_db(state.inner(), |c| {
        c.execute(
            "INSERT INTO worklog(id,job_id,work_on,hours,note) VALUES(?1,?2,?3,?4,?5)",
            rusqlite::params![id, job_id, work_on, hours, note],
        )?;
        crate::audit::log_change(c, "worklog", &id, "insert", "记工时");
        Ok(id)
    })
}

#[tauri::command]
pub fn list_worklogs(
    state: tauri::State<'_, DbState>,
    job_id: String,
) -> Result<Vec<WorklogRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,job_id,work_on,hours,note FROM worklog WHERE job_id=?1 ORDER BY work_on DESC",
        )?;
        let rows = stmt
            .query_map([job_id], |r| {
                Ok(WorklogRow {
                    id: r.get(0)?,
                    job_id: r.get(1)?,
                    work_on: r.get(2)?,
                    hours: r.get(3)?,
                    note: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[derive(Serialize)]
pub struct FilingDraftRow {
    pub id: String,
    pub period: String,
    pub tax_type: String,
    pub amounts: Option<String>,
    pub created_on: String,
}

#[tauri::command]
pub fn list_filing_drafts(
    state: tauri::State<'_, DbState>,
    limit: Option<i64>,
) -> Result<Vec<FilingDraftRow>, String> {
    with_db(state.inner(), |c| {
        let mut stmt = c.prepare(
            "SELECT id,period,tax_type,amounts,created_on FROM filing_draft ORDER BY created_on DESC LIMIT ?1",
        )?;
        let rows = stmt
            .query_map([limit.unwrap_or(50)], |r| {
                Ok(FilingDraftRow {
                    id: r.get(0)?,
                    period: r.get(1)?,
                    tax_type: r.get(2)?,
                    amounts: r.get(3)?,
                    created_on: r.get(4)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    })
}

#[tauri::command]
pub fn verify_vouchers(state: tauri::State<'_, DbState>) -> Result<Vec<crate::maintenance::IntegrityItem>, String> {
    with_db(state.inner(), |c| Ok(crate::maintenance::verify_vouchers(c)))
}

#[tauri::command]
pub fn backup_info(state: tauri::State<'_, DbState>) -> Result<(String, Vec<crate::maintenance::BackupItem>), String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    let last = crate::maintenance::last_auto_backup(conn);
    drop(guard);
    Ok((last, crate::maintenance::list_backups()))
}

#[tauri::command]
pub fn open_logs_dir() -> Result<String, String> {
    let dir = crate::maintenance::logs_dir();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer")
            .arg(dir.as_os_str())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(dir.display().to_string())
}

// ── 离线授权 ───────────────────────────────────────────

#[tauri::command]
pub fn license_status(state: tauri::State<'_, DbState>) -> Result<crate::license::LicenseStatus, String> {
    with_db(state.inner(), |c| Ok(crate::license::status(c)))
}

#[tauri::command]
pub fn license_activate(
    state: tauri::State<'_, DbState>,
    code: String,
) -> Result<crate::license::LicenseStatus, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::license::activate(conn, &code)
}

// ── 全量数据导出 / 换机恢复 ────────────────────────────

#[tauri::command]
pub fn export_all_data(state: tauri::State<'_, DbState>) -> Result<String, String> {
    let dest = crate::intake::pick_folder().ok_or("已取消选择目录")?;
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::maintenance::export_all(conn, std::path::Path::new(&dest))
}

#[tauri::command]
pub fn pick_db_file() -> Option<String> {
    crate::intake::pick_db_file()
}

// ── 经营报表 / 社保公积金 / 更新下载 ─────────────────────

#[tauri::command]
pub fn fin_report(
    state: tauri::State<'_, DbState>,
    start: String,
    end: String,
) -> Result<crate::fin::FinReport, String> {
    with_db(state.inner(), |c| crate::fin::fin_report(c, &start, &end))
}

#[tauri::command]
pub fn si_hf_get(state: tauri::State<'_, DbState>) -> Result<crate::fin::SiHfConfig, String> {
    with_db(state.inner(), |c| Ok(crate::fin::si_hf_config(c)))
}

#[tauri::command]
pub fn si_hf_save(
    state: tauri::State<'_, DbState>,
    config: crate::fin::SiHfConfig,
) -> Result<(), String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::fin::si_hf_save(conn, &config)
}

#[tauri::command]
pub fn si_hf_calc(
    state: tauri::State<'_, DbState>,
    gross: f64,
) -> Result<crate::fin::SiHfResult, String> {
    with_db(state.inner(), |c| Ok(crate::fin::si_hf_calc(c, gross)))
}

#[tauri::command]
pub fn download_update(state: tauri::State<'_, DbState>) -> Result<String, String> {
    let guard = state.inner().0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    crate::maintenance::download_update(conn)
}
