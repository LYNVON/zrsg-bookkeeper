// 统计序列（图表数据）+ 标准申报任务种子 + 正规申报表格
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
pub struct MonthAgg {
    pub month: String, // YYYY-MM
    pub income: f64,
    pub expense: f64,
}

#[derive(Serialize)]
pub struct CatAgg {
    pub category: String,
    pub expense: f64,
}

#[derive(Serialize)]
pub struct StatsSeries {
    pub months: Vec<MonthAgg>,
    pub categories: Vec<CatAgg>,
    pub income_categories: Vec<CatAgg>,
    pub year_income: f64,
    pub year_expense: f64,
    pub month_income: f64,
    pub month_expense: f64,
}

pub fn stats_series(conn: &Connection) -> rusqlite::Result<StatsSeries> {
    let mut stmt = conn.prepare(
        "SELECT substr(occurred_on,1,7) AS m,
                SUM(CASE WHEN direction='income' THEN CAST(amount AS REAL) ELSE 0 END),
                SUM(CASE WHEN direction='expense' THEN CAST(amount AS REAL) ELSE 0 END)
         FROM event GROUP BY m ORDER BY m DESC LIMIT 12",
    )?;
    let mut months: Vec<MonthAgg> = stmt
        .query_map([], |r| {
            Ok(MonthAgg {
                month: r.get(0)?,
                income: r.get(1)?,
                expense: r.get(2)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    months.reverse();

    let mut stmt2 = conn.prepare(
        "SELECT COALESCE(category,'其他'), SUM(CAST(amount AS REAL))
         FROM event WHERE direction='expense' GROUP BY 1 ORDER BY 2 DESC LIMIT 8",
    )?;
    let categories: Vec<CatAgg> = stmt2
        .query_map([], |r| {
            Ok(CatAgg {
                category: r.get(0)?,
                expense: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let mut stmt3 = conn.prepare(
        "SELECT COALESCE(category,'其他'), SUM(CAST(amount AS REAL))
         FROM event WHERE direction='income' GROUP BY 1 ORDER BY 2 DESC LIMIT 8",
    )?;
    let income_categories: Vec<CatAgg> = stmt3
        .query_map([], |r| {
            Ok(CatAgg {
                category: r.get(0)?,
                expense: r.get(1)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;

    let year_income: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='income'",
        [],
        |r| r.get(0),
    )?;
    let year_expense: f64 = conn.query_row(
        "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event WHERE direction='expense'",
        [],
        |r| r.get(0),
    )?;
    let (m_income, m_expense) = months
        .last()
        .map(|m| (m.income, m.expense))
        .unwrap_or((0.0, 0.0));

    Ok(StatsSeries {
        months,
        categories,
        income_categories,
        year_income,
        year_expense,
        month_income: m_income,
        month_expense: m_expense,
    })
}

/// 申报义务调研：按主体类型 / 纳税人类型 / 申报周期 / 是否有雇员，
/// 只生成该公司真正需要报的事项；并在到期日前 30 天到后 420 天窗口内展示。
pub fn rebuild_filing_tasks(conn: &Connection) -> rusqlite::Result<u64> {
    let (subject, taxpayer, cycle, has_emp): (Option<String>, String, String, Option<i64>) = conn
        .query_row(
            "SELECT subject_type, taxpayer_type, filing_cycle, has_employee FROM company LIMIT 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .unwrap_or((None, "small".into(), "quarter".into(), None));

    // 日期工具：以 1970-01-01 为原点换算
    let days_now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        / 86400;
    let today = civil_from_days(days_now);
    let (ty, tm, td) = today;

    // 删除旧的标准化任务，按新画像重建
    conn.execute("DELETE FROM filing_task WHERE id LIKE 'std-%'", [])?;

    let llc = subject.as_deref().unwrap_or("sole") == "llc";
    let monthly_vat = taxpayer == "general" || cycle == "month";
    let general = taxpayer == "general";

    // 收集候选 (due_on, title, channel)
    let mut items: Vec<(String, String, String)> = Vec::new();

    // 增值税（及附加）
    if monthly_vat {
        for y in (ty - 1)..=(ty + 1) {
            for m in 1..=12 {
                items.push((
                    format!("{y:04}-{m:02}-15"),
                    "增值税/附加申报（上月）".into(),
                    "电子税务局".into(),
                ));
            }
        }
    } else {
        // 季报：1/4/7/10 月 15 日
        for y in (ty - 1)..=(ty + 1) {
            for (mm, name) in [(1, "Q4（上年）"), (4, "Q1"), (7, "Q2"), (10, "Q3")] {
                items.push((
                    format!("{y:04}-{mm:02}-15"),
                    format!("增值税/附加季报 {name}"),
                    "电子税务局".into(),
                ));
            }
        }
    }

    if llc {
        // 企业所得税：季度预缴 + 年度汇算
        if !monthly_vat {
            for y in (ty - 1)..=(ty + 1) {
                for (mm, name) in [(1, "Q4（上年）"), (4, "Q1"), (7, "Q2"), (10, "Q3")] {
                    items.push((
                        format!("{y:04}-{mm:02}-15"),
                        format!("企业所得税预缴 {name}"),
                        "电子税务局".into(),
                    ));
                }
            }
        }
        for y in (ty - 1)..=(ty + 1) {
            items.push((
                format!("{y:04}-05-31"),
                "企业所得税汇算清缴".into(),
                "电子税务局".into(),
            ));
        }
    } else {
        // 个体工商户 / 个人独资：经营所得个税
        for y in (ty - 1)..=(ty + 1) {
            for (mm, name) in [(1, "Q4（上年）"), (4, "Q1"), (7, "Q2"), (10, "Q3")] {
                items.push((
                    format!("{y:04}-{mm:02}-15"),
                    format!("经营所得个税预缴 {name}"),
                    "电子税务局".into(),
                ));
            }
            items.push((
                format!("{y:04}-03-31"),
                "经营所得个税汇算（上年度）".into(),
                "电子税务局".into(),
            ));
        }
    }

    // 工商年报（个体户与企业都需）
    for y in (ty - 1)..=(ty + 1) {
        items.push((
            format!("{y:04}-06-30"),
            "工商年报截止".into(),
            "国家企业信用信息公示系统（按注册城市入口）".into(),
        ));
    }

    // 有雇员：每月个税扣缴
    if has_emp.unwrap_or(0) != 0 {
        for y in (ty - 1)..=(ty + 1) {
            for m in 1..=12 {
                items.push((
                    format!("{y:04}-{m:02}-15"),
                    "个人所得税扣缴申报（上月）".into(),
                    "自然人电子税务局（扣缴端）".into(),
                ));
            }
        }
    }

    // 只落到期日前 30 天 ~ 后 420 天窗口内的（避免一堆远期/陈旧提醒）
    let min_d = days_now - 30;
    let max_d = days_now + 420;
    let mut count = 0u64;
    for (i, (due, title, channel)) in items.iter().enumerate() {
        let Some(d) = days_from_yyyy_mm_dd(due) else { continue };
        if d < min_d || d > max_d {
            continue;
        }
        conn.execute(
            "INSERT OR IGNORE INTO filing_task(id,title,due_on,status,channel) VALUES(?1,?2,?3,'pending',?4)",
            rusqlite::params![format!("std-{}-{i}", due), title, due, channel],
        )?;
        count += 1;
    }
    let _ = (ty, tm, td);
    Ok(count)
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

fn days_from_yyyy_mm_dd(s: &str) -> Option<i64> {
    let p: Vec<i64> = s.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 {
        return None;
    }
    Some(days_from_civil(p[0], p[1], p[2]))
}

/// 与 civil_from_days 互逆（1970-01-01 = 0），保证窗口过滤同一标尺
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// 正规申报表格（小规模·简化对照表）：打印后对照填报或下载留存
pub fn render_filing_forms(
    conn: &Connection,
    start: &str,
    end: &str,
) -> Result<String, String> {
    let r = crate::tax::tax_report(conn, start, end).map_err(|e| e.to_string())?;
    let co: String = conn
        .query_row("SELECT name FROM company LIMIT 1", [], |x| x.get(0))
        .unwrap_or_else(|_| "未建档".into());
    let tax_no: String = conn
        .query_row("SELECT tax_no FROM company LIMIT 1", [], |x| x.get(0))
        .unwrap_or_default();
    let income_special = r.income_special;
    let income_general = r.income_general;
    let income_unbilled = r.income_unbilled;
    let income_total = r.income_total;
    let expense_deductible = r.expense_deductible;
    let profit = r.profit;
    let vat_payable = r.vat_payable;
    let vat_detail = r.vat_detail;
    let cit_payable = r.cit_payable;
    let labor_withheld = r.labor_withheld;
    let labor_count = r.labor_count;
    let note = r.note;
    Ok(format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>税务申报表格包</title>
<style>
body{{font-family:'SimSun','Microsoft YaHei',serif;margin:20px;color:#111}}
h1{{text-align:center;letter-spacing:4px;font-size:20px}}
h2{{font-size:15px;border-left:4px solid #2f6bff;padding-left:8px;margin:22px 0 8px}}
table{{width:100%;border-collapse:collapse;font-size:12px}}
th,td{{border:1px solid #333;padding:6px 8px}}
th{{background:#f2f2f2}}
.formhead{{font-size:12px;margin:4px 0 8px}}
.warnbox{{color:#b25a00;border:1px solid #e8c48a;background:#fff8ec;padding:8px;font-size:12px}}
@media print{{body{{margin:10mm}}h2{{page-break-before:always}}}}
</style></head><body>
<h1>税务申报表格包（小规模纳税人）</h1>
<p class="formhead">纳税人：{co}　税号：{tax_no}　所属期：{start} 至 {end}　生成：自然拾光记账</p>

<h2>一、增值税及附加税费申报表（小规模适用）· 数据对照表</h2>
<table>
<tr><th>项目</th><th>行次</th><th>本期数</th><th>填报说明</th></tr>
<tr><td>应税服务销售额（专票）</td><td>1</td><td>{income_special:.2}</td><td>专票税额不免，照常填</td></tr>
<tr><td>应税服务销售额（普票）</td><td>2</td><td>{income_general:.2}</td><td>季销≤30万免征</td></tr>
<tr><td>未开票收入</td><td>3</td><td>{income_unbilled:.2}</td><td>与普票合并计算免征</td></tr>
<tr><td>销售额合计</td><td>4=1+2+3</td><td>{income_total:.2}</td><td></td></tr>
<tr><td>本期应纳税额（1%征收率）</td><td>5</td><td>{vat_payable:.2}</td><td>{vat_detail}</td></tr>
<tr><td>本期应补（退）税额</td><td>6</td><td>{vat_payable:.2}</td><td>填报至电子税务局对应栏</td></tr>
</table>

<h2>二、企业所得税月（季）度预缴纳税申报表（A类）· 数据对照表</h2>
<table>
<tr><th>项目</th><th>行次</th><th>本期金额</th><th>填报说明</th></tr>
<tr><td>营业收入</td><td>1</td><td>{income_total:.2}</td><td></td></tr>
<tr><td>营业成本</td><td>2</td><td>{expense_deductible:.2}</td><td>仅可扣除部分</td></tr>
<tr><td>利润总额</td><td>3</td><td>{profit:.2}</td><td></td></tr>
<tr><td>应纳所得税额（小微 5% 参考）</td><td>4</td><td>{cit_payable:.2}</td><td>年利润≤300万实际税负5%（2023年6/12号）</td></tr>
<tr><td>本期应补（退）所得税额</td><td>5</td><td>{cit_payable:.2}</td><td></td></tr>
</table>

<h2>三、个人所得税扣缴申报 · 劳务报酬数据</h2>
<table>
<tr><th>项目</th><th>金额/数量</th></tr>
<tr><td>本期劳务报酬收入总额</td><td>{labor_count} 笔</td></tr>
<tr><td>本期已预扣个人所得税</td><td>{labor_withheld:.2}</td></tr>
<tr><td>提示</td><td>零税额也需申报（支付方为扣缴义务人）</td></tr>
</table>

<h2>四、申报前自查</h2>
<div class="warnbox">{note}</div>
<ul>
<li>核对从业人数/资产总额（季报附报，与上季差异系统会提示）</li>
<li>发票抬头是否全部正确（错误票不能税前扣除）</li>
<li>无票支出是否已标记「不可扣」并在汇算时调增</li>
</ul>
<p style="font-size:11px;color:#666">自然拾光记账生成 · 数据来自本地账目 · 以电子税务局实际申报表口径为准，如字段与实际表单不一致请按税局表单填报</p>
</body></html>"#
    ))
}
#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (std::path::PathBuf, Connection) {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-filing-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let conn = crate::db::open_and_migrate(&dir.join("f.db")).expect("open");
        (dir, conn)
    }

    fn set_company(conn: &Connection, subject: &str, taxpayer: &str, cycle: &str, has_emp: i64) {
        conn.execute(
            "INSERT INTO company(id,name,tax_no,taxpayer_type,filing_cycle,subject_type,has_employee,created_at,updated_at)
             VALUES('main','甲','T1',?1,?2,?3,?4,'0','0')
             ON CONFLICT(id) DO UPDATE SET taxpayer_type=excluded.taxpayer_type,filing_cycle=excluded.filing_cycle,
               subject_type=excluded.subject_type,has_employee=excluded.has_employee",
            rusqlite::params![taxpayer, cycle, subject, has_emp],
        )
        .unwrap();
    }

    fn titles(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT title FROM filing_task WHERE id LIKE 'std-%' ORDER BY due_on")
            .unwrap();
        stmt.query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap()
    }

    #[test]
    fn sole_trader_gets_business_income_not_cit() {
        let (dir, conn) = ctx();
        set_company(&conn, "sole", "small", "quarter", 0);
        let n = rebuild_filing_tasks(&conn).expect("rebuild");
        assert!(n > 0);
        let ts = titles(&conn);
        assert!(ts.iter().any(|t| t.contains("经营所得个税汇算")), "{ts:?}");
        assert!(!ts.iter().any(|t| t.contains("企业所得税汇算清缴")), "{ts:?}");
        assert!(ts.iter().any(|t| t.contains("工商年报")), "{ts:?}");
        assert!(!ts.iter().any(|t| t.contains("扣缴申报")), "无雇员不应有个税扣缴");
        // 不应有超过 30 天的陈旧提醒
        let mut stmt = conn
            .prepare("SELECT due_on FROM filing_task WHERE id LIKE 'std-%'")
            .unwrap();
        for due in stmt.query_map([], |r| r.get::<_, String>(0)).unwrap().flatten() {
            let p: Vec<i64> = due.split('-').filter_map(|x| x.parse().ok()).collect();
            let days = p[0] * 365 + p[1] * 30 + p[2];
            let now_days = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0)
                / 86400;
            assert!(days >= now_days - 30, "过期提醒不应生成：{due}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn llc_gets_cit_and_employee_withholding() {
        let (dir, conn) = ctx();
        set_company(&conn, "llc", "general", "month", 1);
        rebuild_filing_tasks(&conn).expect("rebuild");
        let ts = titles(&conn);
        assert!(ts.iter().any(|t| t.contains("企业所得税汇算清缴")), "{ts:?}");
        assert!(!ts.iter().any(|t| t.contains("经营所得个税汇算")), "{ts:?}");
        assert!(ts.iter().any(|t| t.contains("增值税/附加申报（上月）")), "一般纳税人应有月报 {ts:?}");
        assert!(ts.iter().any(|t| t.contains("扣缴申报")), "{ts:?}");
        let _ = std::fs::remove_dir_all(dir);
    }
}
