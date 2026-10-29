// 数据层：连接 + 迁移（基线 DDL 内嵌）
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::Mutex;



pub struct DbState(pub Mutex<Option<Connection>>);

const SCHEMA: &str = include_str!("../../../db/schema.sql");

/// 数据目录：exe 旁 data/（便携）→ 不可写时回退 %LOCALAPPDATA%（正式安装）
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("ZRSJ_DB") {
        if let Some(parent) = PathBuf::from(p).parent() {
            return parent.to_path_buf();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let d = dir.join("data");
            if can_write(&d) {
                return d;
            }
        }
    }
    // 回退到用户目录（Program Files 无写权限时）
    let base = std::env::var("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let d = base.join("自然拾光记账").join("data");
    let _ = std::fs::create_dir_all(&d);
    d
}

fn can_write(d: &std::path::Path) -> bool {
    if std::fs::create_dir_all(d).is_err() {
        return false;
    }
    let probe = d.join(".write_test");
    let ok = std::fs::write(&probe, b"ok").is_ok();
    if ok {
        let _ = std::fs::remove_file(&probe);
    }
    ok
}

/// 解析库路径：ZRSJ_DB 环境变量 > exe 旁 data/zrsj.db > 常见开发相对路径
pub fn resolve_db_path() -> PathBuf {
    // 优先级：显式环境变量 > exe 旁 data/zrsj.db > （仅开发构建）旧相对路径
    // 注意：正式版绝不能按「当前工作目录」匹配，否则在任意含 data/zrsj.db 的目录启动
    // 都会打开错误的库（曾导致安装版读到项目里的旧账套）。
    if let Ok(p) = std::env::var("ZRSJ_DB") {
        return PathBuf::from(p);
    }
    let portable = data_dir().join("zrsj.db");
    if portable.exists() {
        return portable;
    }
    if cfg!(debug_assertions) {
        for cand in ["data/zrsj.db", "../data/zrsj.db", "../../data/zrsj.db"] {
            let p = PathBuf::from(cand);
            if p.exists() {
                return p;
            }
        }
    }
    portable
}

pub fn open_and_migrate(path: &std::path::Path) -> rusqlite::Result<Connection> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;

    // 基线 DDL（CREATE IF NOT EXISTS，可重复执行）
    conn.execute_batch(SCHEMA)?;

    // 之后所有结构变更都必须以「迁移」形式追加到 MIGRATIONS，
    // 用 PRAGMA user_version 记录已应用到第几步，便于老库平滑升级。
    run_migrations(&conn)?;

    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('version','1') ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [],
    )?;
    Ok(conn)
}

/// 结构迁移列表：(版本号, SQL)。版本号必须递增且永不复用。
const MIGRATIONS: &[(i64, &str)] = &[
    (
        1,
        "ALTER TABLE voucher ADD COLUMN amount TEXT;
         ALTER TABLE voucher ADD COLUMN issued_on TEXT;
         ALTER TABLE voucher ADD COLUMN seller TEXT;
         ALTER TABLE voucher ADD COLUMN buyer TEXT;",
    ),
    (
        2,
        "ALTER TABLE party ADD COLUMN title TEXT;
         ALTER TABLE party ADD COLUMN hire_on TEXT;
         ALTER TABLE party ADD COLUMN base_salary TEXT;
         ALTER TABLE reimbursement ADD COLUMN party_id TEXT;",
    ),
    (
        3,
        "CREATE TABLE IF NOT EXISTS mail_seen (
           uid TEXT PRIMARY KEY, folder TEXT, seen_at TEXT
         );",
    ),
    (
        4,
        "ALTER TABLE company ADD COLUMN subject_type TEXT;
         ALTER TABLE company ADD COLUMN has_employee INTEGER;",
    ),
];

/// 供加密打开路径复用：跑基线 DDL + 版本迁移
pub fn migrate_schema(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(SCHEMA)?;
    run_migrations(conn)?;
    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('version','1') ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [],
    )?;
    Ok(())
}

fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    let mut current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (ver, sql) in MIGRATIONS {
        if *ver <= current {
            continue;
        }
        // 老库可能已手工补过列，逐条执行并容忍 duplicate column
        for stmt in sql.split(';') {
            let s = stmt.trim();
            if s.is_empty() {
                continue;
            }
            if let Err(e) = conn.execute_batch(s) {
                let msg = e.to_string();
                if !msg.contains("duplicate column") && !msg.contains("already exists") {
                    return Err(e);
                }
            }
        }
        conn.execute_batch(&format!("PRAGMA user_version = {ver}"))?;
        current = *ver;
    }
    Ok(())
}

pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

pub fn new_id() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("id-{nanos:x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path() -> std::path::PathBuf {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-db-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir.join("t.db")
    }

    #[test]
    fn migrations_apply_and_are_idempotent() {
        let path = temp_path();
        let conn = open_and_migrate(&path).expect("first open");
        let ver: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        let last = MIGRATIONS.last().map(|(v, _)| *v).unwrap_or(0);
        assert_eq!(ver, last, "user_version 应等于最后一条迁移");

        // 迁移新增的列与表都在
        let cols: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('voucher') WHERE name IN ('amount','issued_on','seller','buyer')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(cols, 4);
        let mail_seen: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='mail_seen'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(mail_seen, 1);

        drop(conn);
        // 再次打开：不应报错、版本不变
        let conn2 = open_and_migrate(&path).expect("second open");
        let ver2: i64 = conn2.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(ver2, last);
        drop(conn2);
        let _ = std::fs::remove_file(&path);
    }
}
