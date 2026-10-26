// 审计与导入批次：change_log / import_batch
use crate::db::{new_id, now_iso};
use rusqlite::Connection;

/// 记录一次变更（before/after 暂存 detail，便于排查）
pub fn log_change(conn: &Connection, entity: &str, entity_id: &str, action: &str, detail: &str) {
    let _ = conn.execute(
        "INSERT INTO change_log(entity,entity_id,action,after,at) VALUES(?1,?2,?3,?4,?5)",
        rusqlite::params![entity, entity_id, action, detail, now_iso()],
    );
}

/// 记录一个导入批次（file_hash 为幂等键：同一文件重复导入可识别）
pub fn log_batch(
    conn: &Connection,
    source_type: &str,
    file_name: &str,
    file_hash: &str,
    row_count: i64,
) -> String {
    let id = new_id();
    let _ = conn.execute(
        "INSERT OR IGNORE INTO import_batch(id,file_hash,source_type,file_name,row_count,imported_at)
         VALUES(?1,?2,?3,?4,?5,?6)",
        rusqlite::params![id, file_hash, source_type, file_name, row_count, now_iso()],
    );
    id
}

/// 该文件是否已导入过（按 hash）
pub fn batch_seen(conn: &Connection, file_hash: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM import_batch WHERE file_hash=?1",
        [file_hash],
        |r| r.get::<_, i64>(0),
    )
    .unwrap_or(0)
        > 0
}
