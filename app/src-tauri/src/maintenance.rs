// 维护能力：备份列表 / 备份恢复 / 更新源配置与检查 / 许可信息
use crate::db::{data_dir, now_iso};
use rusqlite::Connection;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct BackupItem {
    pub name: String,
    pub path: String,
    pub size: u64,
    pub kind: String, // db | encrypted
    pub created_at: String,
}

pub fn backups_dir() -> PathBuf {
    let d = data_dir().join("backups");
    let _ = std::fs::create_dir_all(&d);
    d
}

pub fn list_backups() -> Vec<BackupItem> {
    let mut out = Vec::new();
    let dir = backups_dir();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            if !p.is_file() {
                continue;
            }
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let kind = if name.ends_with(".zrsjbackup") {
                "encrypted"
            } else if name.ends_with(".db") {
                "db"
            } else {
                continue;
            };
            let meta = e.metadata().ok();
            out.push(BackupItem {
                name: name.clone(),
                path: p.display().to_string(),
                size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                kind: kind.to_string(),
                created_at: meta
                    .and_then(|m| m.modified().ok())
                    .map(|t| {
                        t.duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs().to_string())
                            .unwrap_or_default()
                    })
                    .unwrap_or_default(),
            });
        }
    }
    out.sort_by(|a, b| b.name.cmp(&a.name));
    out
}

fn is_sqlite_file(p: &Path) -> bool {
    let mut buf = [0u8; 16];
    match std::fs::File::open(p) {
        Ok(mut f) => {
            use std::io::Read;
            f.read_exact(&mut buf).is_ok() && &buf == b"SQLite format 3\0"
        }
        Err(_) => false,
    }
}

/// 用给定 SQLite 文件替换当前库（调用前必须先关闭连接）
pub fn replace_db(db_path: &Path, source: &Path) -> Result<(), String> {
    if !source.exists() {
        return Err("备份文件不存在".into());
    }
    if !is_sqlite_file(source) {
        return Err("备份文件不是有效的 SQLite 数据库".into());
    }
    let parent = db_path.parent().ok_or("数据库路径无效")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    // WAL/SHM 必须先清掉，否则旧日志会污染新库
    for suf in ["-wal", "-shm"] {
        let mut side = db_path.as_os_str().to_os_string();
        side.push(suf);
        let _ = std::fs::remove_file(PathBuf::from(side));
    }
    std::fs::copy(source, db_path).map_err(|e| format!("替换数据库失败：{e}"))?;
    Ok(())
}

/// 当前库的即时备份（VACUUM INTO，安全且原子）
pub fn snapshot(conn: &Connection) -> Result<String, String> {
    let out = backups_dir().join(format!("zrsj-{}.db", now_iso()));
    conn.execute(
        &format!(
            "VACUUM INTO '{}'",
            out.display().to_string().replace('\'', "''")
        ),
        [],
    )
    .map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

// ── 更新源（可配置，避免写死占位地址）──────────────────────

pub fn update_source_get(conn: &Connection) -> String {
    conn.query_row(
        "SELECT value FROM schema_meta WHERE key='update_source'",
        [],
        |r| r.get::<_, String>(0),
    )
    .unwrap_or_default()
}

pub fn update_source_save(conn: &Connection, url: &str) -> Result<(), String> {
    let url = url.trim();
    if !url.is_empty() && !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err("更新源必须是 http(s) 地址".into());
    }
    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('update_source',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [url],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct UpdateCheck {
    pub configured: bool,
    pub current: String,
    pub latest: Option<String>,
    pub has_update: bool,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub message: String,
}

pub fn update_check(conn: &Connection) -> Result<UpdateCheck, String> {
    let source = update_source_get(conn);
    let current = env!("CARGO_PKG_VERSION").to_string();
    if source.is_empty() {
        return Ok(UpdateCheck {
            configured: false,
            current,
            latest: None,
            has_update: false,
            notes: None,
            url: None,
            message: "未配置更新源（设置 → 软件更新）".into(),
        });
    }
    let resp = ureq::get(&source)
        .timeout(std::time::Duration::from_secs(10))
        .call()
        .map_err(|e| format!("更新源不可访问：{e}"))?;
    let body = resp
        .into_string()
        .map_err(|e| format!("读取更新信息失败：{e}"))?;
    let v: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("更新信息解析失败：{e}"))?;
    let latest = v
        .get("version")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let notes = v.get("notes").or_else(|| v.get("body")).and_then(|x| x.as_str()).map(|s| s.to_string());
    let url = v
        .get("url")
        .or_else(|| v.get("download"))
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    let has_update = !latest.is_empty() && latest != current;
    Ok(UpdateCheck {
        configured: true,
        current: current.clone(),
        latest: Some(latest.clone()),
        has_update,
        notes,
        url,
        message: if has_update {
            format!("发现新版本 {latest}（当前 {current}）")
        } else {
            "已是最新版本".into()
        },
    })
}

// ── 许可信息（本地只读展示，不做联网校验）──────────────────

#[derive(Serialize)]
pub struct LicenseInfo {
    pub plan: String,
    pub expire_on: Option<String>,
    pub ai_addon: bool,
    pub machine_code: String,
}

fn machine_code() -> String {
    let mut h = Sha256::new();
    h.update(std::env::var("COMPUTERNAME").unwrap_or_default().as_bytes());
    h.update(std::env::var("USERNAME").unwrap_or_default().as_bytes());
    h.update(b"zrsj-machine-v1");
    let d = h.finalize();
    let hex: String = d.iter().take(8).map(|b| format!("{b:02x}")).collect();
    hex.to_uppercase()
}

pub fn license_info(conn: &Connection) -> LicenseInfo {
    let row: Option<(String, Option<String>, i64)> = conn
        .query_row(
            "SELECT plan, expire_on, ai_addon FROM license LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .ok();
    match row {
        Some((plan, expire_on, ai)) => LicenseInfo {
            plan,
            expire_on,
            ai_addon: ai != 0,
            machine_code: machine_code(),
        },
        None => LicenseInfo {
            plan: "trial".into(),
            expire_on: None,
            ai_addon: false,
            machine_code: machine_code(),
        },
    }
}

fn file_sha256(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    let mut h = Sha256::new();
    h.update(&bytes);
    let d = h.finalize();
    let mut s = String::with_capacity(d.len() * 2);
    for b in d.iter() {
        s.push_str(&format!("{b:02x}"));
    }
    Some(s)
}

/// 校验票夹内所有带附件的原件是否与入库哈希一致
pub fn verify_vouchers(conn: &Connection) -> Vec<IntegrityItem> {
    let mut out = Vec::new();
    let mut stmt = match conn.prepare(
        "SELECT id, invoice_no, file_path, sha256 FROM voucher
         WHERE file_path IS NOT NULL AND file_path <> '' ORDER BY created_at DESC LIMIT 500",
    ) {
        Ok(s) => s,
        Err(_) => return out,
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<String>>(3)?,
        ))
    });
    if let Ok(rows) = rows {
        for row in rows.flatten() {
            let (id, no, fp, expect) = row;
            let mut item = IntegrityItem {
                id,
                invoice_no: no,
                file_path: fp.clone(),
                ok: false,
                reason: String::new(),
            };
            match (fp, expect) {
                (Some(f), Some(exp)) if !exp.is_empty() => {
                    let path = Path::new(&f);
                    if !path.exists() {
                        item.reason = "原件文件缺失".into();
                    } else if let Some(actual) = file_sha256(path) {
                        if actual == exp {
                            item.ok = true;
                        } else {
                            item.reason = "原件内容与入库哈希不一致（可能被修改）".into();
                        }
                    } else {
                        item.reason = "原件无法读取".into();
                    }
                }
                (None, _) => item.reason = "无原件路径".into(),
                (_, None) => item.reason = "入库时未记录哈希（旧数据）".into(),
                _ => item.reason = "入库时未记录哈希（旧数据）".into(),
            }
            out.push(item);
        }
    }
    out
}

// ── 备份轮换与自动备份 ─────────────────────────────────

/// 只保留最近 keep 份明文快照
pub fn rotate_snapshots(keep: usize) {
    let dir = backups_dir();
    let mut items: Vec<(String, PathBuf)> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            let p = e.path();
            let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
            if name.starts_with("zrsj-") && name.ends_with(".db") {
                items.push((name, p));
            }
        }
    }
    items.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in items.into_iter().skip(keep) {
        let _ = std::fs::remove_file(p);
    }
}

/// 距上次自动备份超过 24h（或无记录）则做一次，并轮换
pub fn auto_backup_if_due(conn: &Connection) -> Option<String> {
    let last: String = conn
        .query_row("SELECT value FROM schema_meta WHERE key='last_auto_backup'", [], |r| r.get(0))
        .unwrap_or_default();
    let now: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let due = last.parse::<u64>().map(|t| now.saturating_sub(t) >= 86_400).unwrap_or(true);
    if !due {
        return None;
    }
    let path = snapshot(conn).ok()?;
    rotate_snapshots(20);
    let _ = conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('last_auto_backup',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [now.to_string()],
    );
    Some(path)
}

pub fn last_auto_backup(conn: &Connection) -> String {
    conn.query_row("SELECT value FROM schema_meta WHERE key='last_auto_backup'", [], |r| r.get(0))
        .unwrap_or_default()
}

/// 导出全部数据到指定目录：数据库快照 + 全部附件 + 清单
pub fn export_all(conn: &Connection, dest: &Path) -> Result<String, String> {
    let stamp = now_iso();
    let dir = dest.join(format!("自然拾光导出-{stamp}"));
    std::fs::create_dir_all(&dir).map_err(|e| format!("建目录失败：{e}"))?;

    // 1) 数据库快照（含所有表）
    let db_out = dir.join("zrsj.db");
    // 导出为**明文**库，便于在新机直接恢复（工作库本身是加密的）
    crate::cryptdb::export_plaintext(conn, &db_out)?;

    // 2) 附件全量
    let att_src = data_dir().join("attachments");
    let att_dst = dir.join("attachments");
    let mut att_count = 0usize;
    if att_src.exists() {
        copy_tree(&att_src, &att_dst, &mut att_count)?;
    }

    // 3) 清单
    let voucher_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM voucher", [], |r| r.get(0))
        .unwrap_or(0);
    let event_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM event", [], |r| r.get(0))
        .unwrap_or(0);
    let manifest = serde_json::json!({
        "app": "自然拾光记账",
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": stamp,
        "machine_code": crate::license::machine_code(),
        "voucher_count": voucher_count,
        "event_count": event_count,
        "attachment_files": att_count,
        "note": "zrsj.db 为完整账套；attachments 为原件；可在新机用「从文件恢复」导入 zrsj.db"
    });
    let _ = std::fs::write(
        dir.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );

    crate::audit::log_change(conn, "db", "", "merge", &format!("导出全部数据 → {}", dir.display()));
    Ok(dir.display().to_string())
}

fn copy_tree(src: &Path, dst: &Path, counter: &mut usize) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(src).map_err(|e| e.to_string())?.flatten() {
        let p = e.path();
        let name = e.file_name();
        let target = dst.join(&name);
        if p.is_dir() {
            copy_tree(&p, &target, counter)?;
        } else if std::fs::copy(&p, &target).is_ok() {
            *counter += 1;
        }
    }
    Ok(())
}

pub fn logs_dir() -> PathBuf {
    let d = data_dir().join("logs");
    let _ = std::fs::create_dir_all(&d);
    d
}

// ── 原件完整性校验用的数据结构 ─────────────────────────

#[derive(Serialize)]
pub struct IntegrityItem {
    pub id: String,
    pub invoice_no: Option<String>,
    pub file_path: Option<String>,
    pub ok: bool,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_conn() -> (PathBuf, Connection) {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-mnt-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("m.db");
        let conn = crate::db::open_and_migrate(&path).expect("open");
        conn.execute(
            "INSERT INTO company(id,name,tax_no,taxpayer_type,filing_cycle,created_at,updated_at)
             VALUES('main','甲','T1','small','quarter','0','0')",
            [],
        )
        .expect("company");
        (dir, conn)
    }

    #[test]
    fn backup_and_restore_roundtrip() {
        let (dir, conn) = temp_conn();
        let snap = snapshot(&conn).expect("snapshot");
        assert!(Path::new(&snap).exists());

        conn.execute("UPDATE company SET name='乙' WHERE id='main'", []).unwrap();
        let name: String = conn
            .query_row("SELECT name FROM company WHERE id='main'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "乙");

        let db_path = dir.join("m.db");
        // 真实恢复流程会先关闭连接；测试同样先 drop，否则 WAL 会把改动写回
        drop(conn);
        replace_db(&db_path, Path::new(&snap)).expect("replace");
        let conn2 = crate::db::open_and_migrate(&db_path).expect("reopen");
        let restored: String = conn2
            .query_row("SELECT name FROM company WHERE id='main'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(restored, "甲");
        drop(conn2);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn rejects_non_sqlite_file() {
        let (dir, conn) = temp_conn();
        let fake = dir.join("fake.db");
        std::fs::write(&fake, b"not a database").unwrap();
        assert!(replace_db(&dir.join("m.db"), &fake).is_err());
        drop(conn);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn update_source_validation_and_license_default() {
        let (dir, conn) = temp_conn();
        assert_eq!(update_source_get(&conn), "");
        assert!(update_source_save(&conn, "ftp://x").is_err());
        update_source_save(&conn, "https://example.com/latest.json").expect("save");
        assert_eq!(update_source_get(&conn), "https://example.com/latest.json");
        assert!(update_source_save(&conn, "").is_ok());
        let lic = license_info(&conn);
        assert_eq!(lic.plan, "trial");
        assert!(lic.machine_code.len() >= 8);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn integrity_detects_tamper_and_missing() {
        let (dir, conn) = temp_conn();
        let f = dir.join("orig.pdf");
        std::fs::write(&f, b"hello invoice").unwrap();
        let sha = file_sha256(&f).unwrap();
        conn.execute(
            "INSERT INTO voucher(id,kind,invoice_no,status,book_status,reimburse_status,file_path,sha256,is_original,created_at,updated_at)
             VALUES('v1','invoice','NO1','normal','unbooked','none',?1,?2,1,'0','0')",
            rusqlite::params![f.to_string_lossy(), sha],
        )
        .unwrap();

        let ok = verify_vouchers(&conn);
        assert_eq!(ok.len(), 1);
        assert!(ok[0].ok, "should be ok: {}", ok[0].reason);

        std::fs::write(&f, b"tampered content").unwrap();
        let bad = verify_vouchers(&conn);
        assert!(!bad[0].ok);
        assert!(bad[0].reason.contains("不一致"), "reason={}", bad[0].reason);

        std::fs::remove_file(&f).unwrap();
        let missing = verify_vouchers(&conn);
        assert!(!missing[0].ok);
        assert!(missing[0].reason.contains("缺失"), "reason={}", missing[0].reason);

        drop(conn);
        let _ = std::fs::remove_dir_all(dir);
    }
}
/// 下载更新包到临时目录并返回路径（由前端决定是否启动）
pub fn download_update(conn: &Connection) -> Result<String, String> {
    let info = update_check(conn)?;
    if !info.has_update {
        return Err(info.message);
    }
    let url = info.url.ok_or("更新源未提供下载地址")?;
    let out = std::env::temp_dir().join(format!("zrsj-update-{}.exe", now_iso()));
    let resp = ureq::get(&url)
        .timeout(std::time::Duration::from_secs(120))
        .call()
        .map_err(|e| format!("下载失败：{e}"))?;
    let mut file = std::fs::File::create(&out).map_err(|e| format!("写文件失败：{e}"))?;
    std::io::copy(&mut resp.into_reader(), &mut file).map_err(|e| format!("写入失败：{e}"))?;
    crate::audit::log_change(conn, "app", "", "merge", &format!("下载更新包 {url}"));
    Ok(out.display().to_string())
}
