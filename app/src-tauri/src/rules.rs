// 规则沉淀与自动挂靠（PRD §5.4 渐进式自动化）
// 确认行为 → 生成结构化规则 → 新流水进池前先过规则
use crate::db::new_id;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RuleRow {
    pub id: String,
    pub name: Option<String>,
    pub conditions: String,
    pub actions: String,
    pub auto_apply: i64,
    pub enabled: i64,
}

/// 从一次「确认挂靠」沉淀规则：同对方 + 同分类
pub fn learn_from_confirm(conn: &Connection, event_id: &str, tax_status: &str) -> rusqlite::Result<()> {
    let cp: Option<String> = conn
        .query_row("SELECT counterparty_raw FROM event WHERE id=?1", [event_id], |r| r.get(0))
        .ok();
    if let Some(cp) = cp {
        if !cp.is_empty() {
            let conds = format!(
                r#"[{{"field":"counterparty","op":"contains","value":"{}"}}]"#,
                cp.replace('"', "")
            );
            let actions = format!(r#"{{"tax_status":"{tax_status}"}}"#);
            let exists: i64 = conn.query_row(
                "SELECT COUNT(*) FROM rule WHERE conditions=?1",
                [&conds],
                |r| r.get(0),
            )?;
            if exists == 0 {
                conn.execute(
                    "INSERT INTO rule(id,name,conditions,actions,priority,auto_apply,enabled)
                     VALUES(?1,?2,?3,?4,100,0,1)",
                    rusqlite::params![new_id(), format!("确认规则：{cp}"), conds, actions],
                )?;
            }
        }
    }
    Ok(())
}

pub fn list_rules(conn: &Connection) -> rusqlite::Result<Vec<RuleRow>> {
    let mut stmt = conn.prepare("SELECT id,name,conditions,actions,auto_apply,enabled FROM rule ORDER BY priority LIMIT 100")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(RuleRow {
                id: r.get(0)?,
                name: r.get(1)?,
                conditions: r.get(2)?,
                actions: r.get(3)?,
                auto_apply: r.get(4)?,
                enabled: r.get(5)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(rows)
}

pub fn toggle_rule(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute("UPDATE rule SET auto_apply = CASE auto_apply WHEN 1 THEN 0 ELSE 1 END WHERE id=?1", [id])?;
    Ok(())
}

/// 规则命中：把 auto_apply 规则套到未定税务状态的事项上（启动/导入后调用）
pub fn apply_rules(conn: &Connection) -> rusqlite::Result<u64> {
    let mut n = 0u64;
    let mut stmt = conn.prepare("SELECT conditions, actions FROM rule WHERE enabled=1 AND auto_apply=1")?;
    let rules: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    for (conds, actions) in rules {
        let cs: Vec<serde_json::Value> = serde_json::from_str(&conds).unwrap_or_default();
        let act: serde_json::Value = serde_json::from_str(&actions).unwrap_or_default();
        let tax_status = act.get("tax_status").and_then(|v| v.as_str()).unwrap_or("可扣");
        if let Some(c0) = cs.first() {
            if c0.get("field").and_then(|v| v.as_str()) == Some("counterparty") {
                if let Some(val) = c0.get("value").and_then(|v| v.as_str()) {
                    let changed = conn.execute(
                        "UPDATE event SET tax_status=?1
                         WHERE (tax_status IS NULL OR tax_status='待补票')
                           AND counterparty_raw LIKE '%' || ?2 || '%'",
                        rusqlite::params![tax_status, val],
                    )?;
                    n += changed as u64;
                }
            }
        }
    }
    Ok(n)
}
