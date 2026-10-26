// 税费计算与季报底稿（PRD §5.7，v1.6 口径：专票不免/普票免征/未开票合并）
// 参数外置：阈值/税率集中在本文件顶部
use rusqlite::Connection;
use serde::Serialize;

pub const VAT_THRESHOLD: f64 = 300_000.0; // 季销免征线（普票+未开票），专票不免
pub const CIT_THRESHOLD: f64 = 3_000_000.0;
pub const CIT_RATE_SMALL: f64 = 0.05; // 小微实际税负（2023 年 6/12 号，有效至 2027-12-31）

#[derive(Serialize)]
pub struct TaxReport {
    pub period: String,
    pub income_special: f64,  // 开票（专票口径，暂按有票=专票最保守；普票单列）
    pub income_general: f64,
    pub income_unbilled: f64,
    pub income_total: f64,
    pub expense_total: f64,
    pub expense_deductible: f64, // 可扣+收据可扣
    pub expense_nodeduct: f64,   // 不可扣 → 汇算调增
    pub profit: f64,
    pub vat_payable: f64,
    pub vat_detail: String,
    pub cit_payable: f64,
    pub labor_withheld: f64,
    pub labor_count: i64,
    pub note: String,
}

/// 劳务预扣（61 号公告三级累进）
pub fn labor_withhold(income: f64) -> f64 {
    if income <= 800.0 {
        return 0.0;
    }
    let taxable = if income <= 4000.0 { income - 800.0 } else { income * 0.8 };
    let (rate, quick) = if taxable > 50000.0 {
        (0.4, 7000.0)
    } else if taxable > 20000.0 {
        (0.3, 2000.0)
    } else {
        (0.2, 0.0)
    };
    ((taxable * rate - quick) * 100.0).round() / 100.0
}

/// 工资薪金累计预扣预缴（年度税率表，3%/10%/…/45% 七级，超额累进）
/// 输入：本年截至上月的累计（gross/si/tax）+ 本月数；输出本月应预扣
pub fn wage_withhold_cumulative(
    cum_gross: f64,
    cum_si: f64,
    cum_tax_paid: f64,
    months: u32, // 本年截至本月的月数
    gross: f64,
    si_hf: f64,
) -> f64 {
    let taxable_cum = cum_gross + gross - 5000.0 * months as f64 - (cum_si + si_hf);
    if taxable_cum <= 0.0 {
        return 0.0;
    }
    let (rate, quick) = match taxable_cum {
        t if t <= 36000.0 => (0.03, 0.0),
        t if t <= 144000.0 => (0.10, 2520.0),
        t if t <= 300000.0 => (0.20, 16920.0),
        t if t <= 420000.0 => (0.25, 31920.0),
        t if t <= 660000.0 => (0.30, 52920.0),
        t if t <= 960000.0 => (0.35, 85920.0),
        _ => (0.45, 181920.0),
    };
    let tax_cum = taxable_cum * rate - quick;
    ((tax_cum - cum_tax_paid).max(0.0) * 100.0).round() / 100.0
}

/// 期间报表：start/end 为 YYYY-MM-DD
pub fn tax_report(conn: &Connection, start: &str, end: &str) -> rusqlite::Result<TaxReport> {
    let sum = |sql: &str| -> rusqlite::Result<f64> {
        conn.query_row(sql, rusqlite::params![start, end], |r| r.get(0))
    };
    // 开票：已挂发票凭证的 income；未开票：tax_status='未开票收入'
    let income_billed = sum(
        "SELECT COALESCE(SUM(CAST(e.amount AS REAL)),0) FROM event e
         WHERE e.direction='income' AND e.occurred_on BETWEEN ?1 AND ?2
           AND EXISTS (SELECT 1 FROM voucher v WHERE v.event_id=e.id AND v.invoice_no IS NOT NULL)",
    )?;
    let income_unbilled = sum(
        "SELECT COALESCE(SUM(CAST(e.amount AS REAL)),0) FROM event e
         WHERE e.direction='income' AND e.occurred_on BETWEEN ?1 AND ?2
           AND (e.tax_status='未开票收入' OR NOT EXISTS (SELECT 1 FROM voucher v WHERE v.event_id=e.id AND v.invoice_no IS NOT NULL))",
    )?;
    // 保守口径：有票=专票（不免）；无票=普票/未开票合并免征——PRD v1.6 拆分落地
    let income_special = income_billed;
    let income_general = 0.0;
    let expense_total = sum(
        "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event
         WHERE direction='expense' AND occurred_on BETWEEN ?1 AND ?2
           AND (tax_status IS NULL OR tax_status NOT IN ('不涉税'))",
    )?;
    let expense_deductible = sum(
        "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event
         WHERE direction='expense' AND occurred_on BETWEEN ?1 AND ?2
           AND tax_status IN ('可扣','收据可扣')",
    )?;
    let expense_nodeduct = sum(
        "SELECT COALESCE(SUM(CAST(amount AS REAL)),0) FROM event
         WHERE direction='expense' AND occurred_on BETWEEN ?1 AND ?2
           AND tax_status='不可扣'",
    )?;

    let income_total = income_special + income_general + income_unbilled;
    let profit = income_total - expense_deductible;

    // 增值税（小规模，1%）：≤30万 → 普票+未开票免征、专票不免
    let (vat_payable, vat_detail) = if income_total <= VAT_THRESHOLD {
        let sp = (income_special * 0.01 * 100.0).round() / 100.0;
        (sp, format!("季销 {:.2}≤30万：普票与未开票免征，专票税额 {:.2} 不免", income_total, sp))
    } else {
        let v = (income_total * 0.01 * 100.0).round() / 100.0;
        (v, format!("季销 {:.2}>30万：全额计征 1% = {:.2}", income_total, v))
    };

    // 企税（年利润口径粗算：本期间利润×小微率作预缴参考）
    let cit_payable = if profit <= 0.0 {
        0.0
    } else if profit <= CIT_THRESHOLD {
        (profit * CIT_RATE_SMALL * 100.0).round() / 100.0
    } else {
        (profit * 0.25 * 100.0).round() / 100.0
    };

    // 劳务预扣汇总（WITHHOLD_LEDGER 优先，缺则按 tax_status 估算不可用——此处只汇总台账）
    let labor_withheld = conn.query_row(
        "SELECT COALESCE(SUM(CAST(withheld AS REAL)),0) FROM withhold_ledger WHERE paid_on BETWEEN ?1 AND ?2",
        rusqlite::params![start, end],
        |r| r.get(0),
    )?;
    let labor_count = conn.query_row(
        "SELECT COUNT(*) FROM withhold_ledger WHERE paid_on BETWEEN ?1 AND ?2",
        rusqlite::params![start, end],
        |r| r.get(0),
    )?;

    let note = if expense_nodeduct > 0.0 {
        format!("⚠ 无票不可扣支出 {:.2} 元，汇算清缴需纳税调增", expense_nodeduct)
    } else {
        "本期无不可扣支出".into()
    };

    Ok(TaxReport {
        period: format!("{start} ~ {end}"),
        income_special,
        income_general,
        income_unbilled,
        income_total,
        expense_total,
        expense_deductible,
        expense_nodeduct,
        profit,
        vat_payable,
        vat_detail,
        cit_payable,
        labor_withheld,
        labor_count,
        note,
    })
}

/// 季报底稿 HTML（对齐综合关联式申报字段顺序，打印/存档）
pub fn render_draft_html(r: &TaxReport, company: &str) -> String {
    format!(
        r#"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>季报底稿</title>
<style>body{{font-family:'SimSun',serif;margin:24px;color:#111}}h1{{text-align:center;letter-spacing:6px}}
table{{width:100%;border-collapse:collapse;font-size:13px}}th,td{{border:1px solid #444;padding:8px}}
th{{background:#f0f0f0}}.warn{{color:#b25a00}}@media print{{body{{margin:12mm}}}}</style></head><body>
<h1>季度申报底稿</h1>
<p>主体：{company}　期间：{period}</p>
<table>
<tr><th>项目</th><th>金额（元）</th><th>说明</th></tr>
<tr><td>开票收入（按专票口径，税额不免）</td><td>{income_special:.2}</td><td>已挂发票的收入</td></tr>
<tr><td>普票收入（免征）</td><td>{income_general:.2}</td><td></td></tr>
<tr><td>未开票收入（与普票合并免征）</td><td>{income_unbilled:.2}</td><td>季报时并入申报</td></tr>
<tr><td><b>收入合计</b></td><td><b>{income_total:.2}</b></td><td></td></tr>
<tr><td>费用支出合计</td><td>{expense_total:.2}</td><td></td></tr>
<tr><td>其中可扣除</td><td>{expense_deductible:.2}</td><td>可扣+收据可扣</td></tr>
<tr><td>不可扣除</td><td>{expense_nodeduct:.2}</td><td>汇算调增</td></tr>
<tr><td>本期利润</td><td>{profit:.2}</td><td>收入−可扣支出</td></tr>
<tr><td>应缴增值税</td><td>{vat_payable:.2}</td><td>{vat_detail}</td></tr>
<tr><td>应预缴企业所得税（参考）</td><td>{cit_payable:.2}</td><td>小微 5%（≤300 万，2023 年 6/12 号）</td></tr>
<tr><td>劳务报酬已预扣个税</td><td>{labor_withheld:.2}</td><td>{labor_count} 笔，扣缴申报草稿数据</td></tr>
</table>
<p class="warn">{note}</p>
<p style="font-size:11px;color:#666;margin-top:18px">自然拾光记账生成 · 填报以电子税务局综合关联式申报为准 · 重大事项咨询主管税局</p>
</body></html>"#,
        period = r.period,
        company = company,
        income_special = r.income_special,
        income_general = r.income_general,
        income_unbilled = r.income_unbilled,
        income_total = r.income_total,
        expense_total = r.expense_total,
        expense_deductible = r.expense_deductible,
        expense_nodeduct = r.expense_nodeduct,
        profit = r.profit,
        vat_payable = r.vat_payable,
        vat_detail = r.vat_detail,
        cit_payable = r.cit_payable,
        labor_withheld = r.labor_withheld,
        labor_count = r.labor_count,
        note = r.note,
    )
}
