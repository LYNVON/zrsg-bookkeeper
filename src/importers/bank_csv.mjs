// 银行流水 CSV 导入器（PRD §5.1）：列映射向导的数据层
// parse(csvText, mapping) → { rows[], warnings[] }
// mapping: { date, amount, income, expense, counterparty, memo, direction? }
//   - 金额：amount 单列（正负表方向）或 income/expense 双列
//   - 日期格式自动识别：YYYY-MM-DD / YYYY/MM/DD / YYYYMMDD / 2026年10月8日

const DATE_RE = [
  { re: /^(\d{4})-(\d{1,2})-(\d{1,2})/, f: (m) => `${m[1]}-${pad(m[2])}-${pad(m[3])}` },
  { re: /^(\d{4})\/(\d{1,2})\/(\d{1,2})/, f: (m) => `${m[1]}-${pad(m[2])}-${pad(m[3])}` },
  { re: /^(\d{4})年(\d{1,2})月(\d{1,2})日/, f: (m) => `${m[1]}-${pad(m[2])}-${pad(m[3])}` },
];

function pad(s) { return String(s).padStart(2, '0'); }

export function parseDate(raw) {
  const s = String(raw ?? '').trim();
  for (const { re, f } of DATE_RE) {
    const m = s.match(re);
    if (m) return f(m);
  }
  const d8 = s.match(/^(\d{4})(\d{2})(\d{2})$/);
  if (d8) return `${d8[1]}-${d8[2]}-${d8[3]}`;
  return null;
}

export function parseAmount(raw) {
  const s = String(raw ?? '').replace(/[¥,，\s]/g, '').replace(/元$/, '');
  if (!s || s === '-' || s === '--') return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

/** 简单 CSV 拆分（支持引号包裹含逗号字段） */
export function splitCsvLine(line) {
  const out = [];
  let cur = '', inQ = false;
  for (let i = 0; i < line.length; i++) {
    const c = line[i];
    if (c === '"') {
      if (inQ && line[i + 1] === '"') { cur += '"'; i++; }
      else inQ = !inQ;
    } else if ((c === ',' || c === '，' || c === '\t') && !inQ) {
      out.push(cur.trim()); cur = '';
    } else cur += c;
  }
  out.push(cur.trim());
  return out;
}

/** 常见银行列名 → 标准字段 的启发式识别（映射向导首次自动猜，之后记住模板） */
export function guessMapping(headerCells) {
  const g = {};
  headerCells.forEach((h, i) => {
    const s = String(h);
    if (/日期|交易日期|记账日期|value.?date/i.test(s)) g.date ??= i;
    if (/对方|对手|户名|收款人|付款人|payee/i.test(s)) g.counterparty ??= i;
    if (/摘要|用途|事由|附言|备注|description|memo/i.test(s)) g.memo ??= i;
    if (/收入|存入|贷方发生/i.test(s)) g.income ??= i;
    if (/支出|取出|借方发生/i.test(s)) g.expense ??= i;
    if (/金额|发生额|amount/i.test(s) && g.amount == null) g.amount = i;
  });
  return g;
}

/**
 * 解析流水 CSV
 * @param {string} text CSV 全文
 * @param {object} mapping 列号映射（0-based）或 null=自动猜（首行为表头）
 */
export function parseBankCsv(text, mapping = null) {
  const lines = text.split(/\r?\n/).filter(l => l.trim());
  if (!lines.length) return { rows: [], warnings: ['空文件'], mapping: null };

  let map = mapping;
  let start = 0;
  if (!map) {
    map = guessMapping(splitCsvLine(lines[0]));
    start = 1; // 跳过表头
    if (map.date == null) {
      // 无表头：假设列序 日期,对方,摘要,收入,支出
      map = { date: 0, counterparty: 1, memo: 2, income: 3, expense: 4 };
      start = 0;
    }
  }
  const warnings = [];
  if (map.date == null) warnings.push('未识别日期列，请在映射向导中指定');
  if (map.amount == null && map.income == null && map.expense == null)
    warnings.push('未识别金额列，请在映射向导中指定');

  const rows = [];
  for (let i = start; i < lines.length; i++) {
    const cells = splitCsvLine(lines[i]);
    const occurred_on = map.date != null ? parseDate(cells[map.date]) : null;
    let amount = null, direction = map.direction;
    if (map.amount != null) {
      amount = parseAmount(cells[map.amount]);
      if (amount != null && amount < 0) { amount = Math.abs(amount); direction ??= 'expense'; }
      else if (amount != null) direction ??= 'income';
    } else {
      const inc = parseAmount(map.income != null ? cells[map.income] : null);
      const exp = parseAmount(map.expense != null ? cells[map.expense] : null);
      if (inc != null && inc !== 0) { amount = inc; direction ??= 'income'; }
      else if (exp != null && exp !== 0) { amount = exp; direction ??= 'expense'; }
    }
    if (!occurred_on || amount == null) {
      warnings.push(`第 ${i + 1} 行解析失败（日期/金额），已跳过`);
      continue;
    }
    rows.push({
      occurred_on,
      direction: direction ?? 'expense',
      amount: amount.toFixed(2),
      counterparty_raw: map.counterparty != null ? cells[map.counterparty] : null,
      memo: map.memo != null ? cells[map.memo] : null,
      source_line: i + 1,
    });
  }
  return { rows, warnings, mapping: map };
}
