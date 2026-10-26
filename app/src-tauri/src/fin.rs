// 简版经营报表 + 社保公积金测算
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct MonthRow {
    pub month: String,
    pub income: f64,
    pub expense: f64,
    pub profit: f64,
}

#[derive(Serialize)]
pub struct CatRow {
    pub category: String,
    pub amount: f64,
}

#[derive(Serialize)]
pub struct AccountRow {
    pub name: String,
    pub balance: f64,
}

#[derive(Serialize)]
pub struct FinReport {
    pub start: String,
    pub end: String,
    pub months: Vec<MonthRow>,
    pub income_by_cat: Vec<CatRow>,
    pub expense_by_cat: Vec<CatRow>,
    pub accounts: Vec<AccountRow>,
    pub total_income: f64,
    pub total_expense: f64,
    pub total_profit: f64,
    pub ar: f64,
    pub ap: f64,
    pub note: String,
}

fn f(v: Option<String>) -> f64 {
    v.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0)
}

/// 经营收支报表：按流水汇总（收付实现制），不是会计准则的利润表
pub fn fin_report(conn: &Connection, start: &str, end: &str) -> rusqlite::Result<FinReport> {
    // 月度收支
    let mut stmt = conn.prepare(
        "SELECT substr(occurred_on,1,7) AS m, direction, SUM(CAST(amount AS REAL))
         FROM event
         WHERE occurred_on >= ?1 AND occurred_on <= ?2
           AND direction IN ('income','expense')
         GROUP BY m, direction ORDER BY m",
    )?;
    let rows = stmt
        .query_map([start, end], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut months: Vec<MonthRow> = Vec::new();
    for (m, dir, amt) in rows {
        if let Some(last) = months.last_mut().filter(|x| x.month == m) {
            if dir == "income" {
                last.income += amt;
            } else {
                last.expense += amt;
            }
            last.profit = last.income - last.expense;
        } else {
            let (inc, exp) = if dir == "income" { (amt, 0.0) } else { (0.0, amt) };
            months.push(MonthRow { month: m, income: inc, expense: exp, profit: inc - exp });
        }
    }

    // 分类汇总
    let mut by_cat = |dir: &str| -> rusqlite::Result<Vec<CatRow>> {
        let mut s = conn.prepare(
            "SELECT COALESCE(NULLIF(category,''),'未分类'), SUM(CAST(amount AS REAL))
             FROM event WHERE occurred_on >= ?1 AND occurred_on <= ?2 AND direction = ?3
             GROUP BY 1 ORDER BY 2 DESC",
        )?;
        let v = s
            .query_map([start, end, dir], |r| {
                Ok(CatRow { category: r.get(0)?, amount: r.get(1)? })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(v)
    };
    let income_by_cat = by_cat("income")?;
    let expense_by_cat = by_cat("expense")?;

    // 账户余额：期初 + 全部历史收支
    let mut stmt = conn.prepare(
        "SELECT a.name,
                COALESCE(CAST(a.opening_balance AS REAL),0)
                + COALESCE((
                    SELECT SUM(CASE e.direction
                                 WHEN 'income' THEN CAST(e.amount AS REAL)
                                 WHEN 'expense' THEN -CAST(e.amount AS REAL)
                                 ELSE 0 END)
                    FROM event e WHERE e.account_id = a.id
                  ),0)
         FROM account a ORDER BY a.created_at",
    )?;
    let accounts = stmt
        .query_map([], |r| Ok(AccountRow { name: r.get(0)?, balance: r.get(1)? }))?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let total_income: f64 = income_by_cat.iter().map(|c| c.amount).sum();
    let total_expense: f64 = expense_by_cat.iter().map(|c| c.amount).sum();

    // 应收应付（复用 ar_ap 逻辑的简化版）
    let ar: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event
             WHERE direction='income' AND settle_status<>'settled' AND tax_status IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let ap: f64 = conn
        .query_row(
            "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event
             WHERE direction='expense' AND settle_status<>'settled' AND tax_status IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let _ = f(None);
    Ok(FinReport {
        start: start.to_string(),
        end: end.to_string(),
        months,
        income_by_cat,
        expense_by_cat,
        accounts,
        total_income,
        total_expense,
        total_profit: total_income - total_expense,
        ar,
        ap,
        note: "按实收实付汇总，用于经营判断；不作为会计利润表/纳税申报依据".into(),
    })
}

// ── 社保公积金 ────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct SiHfConfig {
    pub base_min: f64,
    pub base_max: f64,
    pub pension: f64,
    pub medical: f64,
    pub unemploy: f64,
    pub housing: f64,
}

impl Default for SiHfConfig {
    fn default() -> Self {
        Self {
            base_min: 2300.0,
            base_max: 30000.0,
            pension: 0.08,
            medical: 0.02,
            unemploy: 0.005,
            housing: 0.05,
        }
    }
}

pub fn si_hf_config(conn: &Connection) -> SiHfConfig {
    conn.query_row("SELECT value FROM schema_meta WHERE key='si_hf_config'", [], |r| {
        r.get::<_, String>(0)
    })
    .ok()
    .and_then(|s| serde_json::from_str(&s).ok())
    .unwrap_or_default()
}

pub fn si_hf_save(conn: &Connection, cfg: &SiHfConfig) -> Result<(), String> {
    let json = serde_json::to_string(cfg).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('si_hf_config',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize)]
pub struct SiHfResult {
    pub base: f64,
    pub pension: f64,
    pub medical: f64,
    pub unemploy: f64,
    pub housing: f64,
    pub total: f64,
    pub net: f64,
}

pub fn si_hf_calc(conn: &Connection, gross: f64) -> SiHfResult {
    let c = si_hf_config(conn);
    let base = gross.clamp(c.base_min, c.base_max);
    let pension = base * c.pension;
    let medical = base * c.medical;
    let unemploy = base * c.unemploy;
    let housing = base * c.housing;
    let total = pension + medical + unemploy + housing;
    SiHfResult {
        base,
        pension,
        medical,
        unemploy,
        housing,
        total,
        net: (gross - total).max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (std::path::PathBuf, Connection) {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-fin-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let conn = crate::db::open_and_migrate(&dir.join("f.db")).expect("open");
        (dir, conn)
    }

    #[test]
    fn report_groups_months_and_categories() {
        let (dir, conn) = ctx();
        conn.execute(
            "INSERT INTO account(id,name,type,is_company,opening_balance,created_at)
             VALUES('acc1','公户','bank',1,'0','0')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,account_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,created_at,updated_at)
             VALUES('e1','2026-01-05','income','1000',NULL,'acc1','none',NULL,'服务','manual',NULL,'none','none','0','0'),
                    ('e2','2026-01-20','expense','300',NULL,'acc1','none',NULL,'餐饮','manual',NULL,'none','none','0','0'),
                    ('e3','2026-02-10','income','500',NULL,'acc1','none',NULL,'服务','manual',NULL,'none','none','0','0')",
            [],
        )
        .unwrap();
        let r = fin_report(&conn, "2026-01-01", "2026-02-28").expect("report");
        assert_eq!(r.months.len(), 2);
        assert!((r.total_income - 1500.0).abs() < 0.01);
        assert!((r.total_expense - 300.0).abs() < 0.01);
        assert!((r.total_profit - 1200.0).abs() < 0.01);
        assert_eq!(r.income_by_cat[0].category, "服务");
        drop(conn);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn si_hf_respects_base_bounds() {
        let (dir, conn) = ctx();
        let low = si_hf_calc(&conn, 1000.0);
        assert!((low.base - 2300.0).abs() < 0.01, "低于下限按下限");
        let high = si_hf_calc(&conn, 50000.0);
        assert!((high.base - 30000.0).abs() < 0.01, "高于上限按上限");
        let mid = si_hf_calc(&conn, 10000.0);
        assert!((mid.total - (10000.0 * (0.08 + 0.02 + 0.005 + 0.05))).abs() < 0.01);
        drop(conn);
        let _ = std::fs::remove_dir_all(dir);
    }
}
