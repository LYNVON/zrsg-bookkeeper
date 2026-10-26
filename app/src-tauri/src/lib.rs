mod assistant;
mod audit;
mod commands;
mod crypto;
mod fin;
mod cryptdb;
mod db;
mod import_doc;
mod intake;
mod license;
mod mail;
mod maintenance;
mod match_engine;
mod ops;
mod report;
mod rules;
mod tax;
mod vault;

use db::DbState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // 单实例：第二次启动只唤起已有窗口，避免两个进程同时写同一个 SQLite
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(DbState(std::sync::Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![
            commands::init_app,
            commands::db_ping,
            commands::add_event,
            commands::list_events,
            commands::list_tasks,
            commands::toggle_task,
            commands::dashboard,
            commands::save_company,
            commands::get_company,
            commands::backup_now,
            commands::import_bank_csv,
            commands::list_unmatched,
            commands::match_suggestions,
            commands::confirm_match,
            commands::apply_disposition,
            commands::due_tasks,
            commands::list_vouchers,
            commands::add_voucher,
            commands::create_reimbursement,
            commands::mark_reimbursed,
            commands::list_reimbursements,
            commands::export_document,
            commands::reimbursement_lines,
            commands::tax_report,
            commands::export_tax_draft,
            commands::list_parties,
            commands::add_party,
            commands::list_jobs,
            commands::add_job,
            commands::job_pay,
            commands::job_settle,
            commands::add_withhold,
            commands::list_employees,
            commands::add_employee,
            commands::wage_tax_preview,
            commands::add_payroll,
            commands::list_payroll,
            commands::add_bonus,
            commands::list_accounts,
            commands::add_account,
            commands::account_events,
            commands::transfer,
            commands::update_account,
            commands::delete_account,
            commands::account_transfers,
            commands::export_text,
            commands::update_event,
            commands::delete_event,
            commands::list_rules,
            commands::toggle_rule,
            commands::apply_rules,
            commands::ar_ap,
            commands::settle_event,
            commands::biz_brief,
            commands::fetch_invoices,
            commands::import_voucher_files,
            commands::pick_voucher_files,
            commands::mail_config_get,
            commands::mail_config_save,
            commands::mail_config_clear,
            commands::fetch_invoices_saved,
            commands::open_voucher_file,
            commands::backups_list,
            commands::restore_backup,
            commands::restore_encrypted,
            commands::update_source_get,
            commands::update_source_save,
            commands::update_check,
            commands::license_info,
            commands::add_worklog,
            commands::list_worklogs,
            commands::list_filing_drafts,
            commands::verify_vouchers,
            commands::backup_info,
            commands::open_logs_dir,
            commands::license_status,
            commands::license_activate,
            commands::export_all_data,
            commands::pick_db_file,
            commands::fin_report,
            commands::si_hf_get,
            commands::si_hf_save,
            commands::si_hf_calc,
            commands::download_update,
            commands::parse_external_doc,
            commands::ai_chat,
            commands::ai_rule_pack,
            commands::backup_encrypted,
            commands::verify_encrypted,
            commands::stats_series,
            commands::seed_default_tasks,
            commands::export_filing_forms,
            commands::search_all,
        ])
        .setup(|app| {
            // 崩溃写入 data/logs/crash.log，便于排查（发布版 panic=abort 也保留）
            std::panic::set_hook(Box::new(|info| {
                use std::io::Write;
                let dir = crate::maintenance::logs_dir();
                let line = format!("[{}] {info}\n", crate::db::now_iso());
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("crash.log"))
                {
                    let _ = f.write_all(line.as_bytes());
                }
            }));
            let state = app.state::<DbState>();
            if let Err(e) = commands::init_app(app.state::<DbState>()) {
                use std::io::Write;
                let dir = crate::maintenance::logs_dir();
                let line = format!("[{}] init_app failed: {e}\n", crate::db::now_iso());
                if let Ok(mut f) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(dir.join("init-error.log"))
                {
                    let _ = f.write_all(line.as_bytes());
                }
            }
            // 启动时按需自动备份（24h 一次，保留最近 20 份）
            if let Ok(guard) = state.0.lock() {
                if let Some(conn) = guard.as_ref() {
                    let _ = crate::maintenance::auto_backup_if_due(conn);
                }
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
