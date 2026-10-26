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

/// 标准申报任务种子：季报/年报/汇算（按当前年生成），已有任务则跳过
pub fn seed_default_tasks(conn: &Connection) -> rusqlite::Result<u64> {
    let n: i64 = conn.query_row("SELECT COUNT(*) FROM filing_task", [], |r| r.get(0))?;
    if n > 0 {
        return Ok(0);
    }
    let year: i64 = {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let days = (t / 86400) as i64;
        1970 + days / 365 // 粗略年份，仅用于默认任务命名
    };
    let mut count = 0u64;
    let items: Vec<(String, String, String)> = vec![
        (format!("{year}-01-15"), "Q4 增值税/企税季报（上年）".into(), "电子税务局·综合关联式申报".into()),
        (format!("{year}-04-15"), "Q1 增值税/企税季报".into(), "电子税务局·综合关联式申报".into()),
        (format!("{year}-04-30"), "工商年报开始（截止 6/30）".into(), "国家企业信用信息公示系统（按注册城市入口）".into()),
        (format!("{year}-05-31"), "企业所得税汇算清缴".into(), "电子税务局".into()),
        (format!("{year}-06-30"), "工商年报截止".into(), "国家企业信用信息公示系统（按注册城市入口）".into()),
        (format!("{year}-07-15"), "Q2 增值税/企税季报".into(), "电子税务局·综合关联式申报".into()),
        (format!("{year}-10-15"), "Q3 增值税/企税季报".into(), "电子税务局·综合关联式申报".into()),
    ];
    for (i, (due, title, channel)) in items.iter().enumerate() {
        conn.execute(
            "INSERT INTO filing_task(id,title,due_on,status,channel) VALUES(?1,?2,?3,'pending',?4)",
            rusqlite::params![format!("std-{year}-{i}"), title, due, channel],
        )?;
        count += 1;
    }
    Ok(count)
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
