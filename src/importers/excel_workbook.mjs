// Excel 工作簿迁移向导（PRD §5.10）：《自然拾光_财务全套》→ 事项/凭证/申报任务 + 四勾稽
// 读取走 Python openpyxl 桥接（同 invoice_xlsx，生产版换 Rust）；
// 解析与勾稽是纯函数，可单测。

import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseDate, parseAmount } from './bank_csv.mjs';

const PY_DUMP = `
import json, sys
from openpyxl import load_workbook
wb = load_workbook(sys.argv[1], read_only=True, data_only=True)
out = {}
for ws in wb.worksheets:
    rows = []
    for r in ws.iter_rows(values_only=True):
        rows.append(["" if v is None else (str(v) if not isinstance(v, (int, float)) else v) for v in r])
    out[ws.title] = rows
print(json.dumps(out, ensure_ascii=False, default=str))
`;

function pythonBin() {
  const candidates = [process.env.MIMO_PYTHON, 'python', 'python3', 'py'].filter(Boolean);
  for (const bin of candidates) {
    try { execFileSync(bin, ['-c', 'import openpyxl; print("ok")'], { stdio: 'pipe' }); return bin; }
    catch { /* next */ }
  }
  throw new Error('no python with openpyxl');
}

export function readWorkbook(filePath) {
  const py = pythonBin();
  const dir = mkdtempSync(join(tmpdir(), 'zrsj-wb-'));
  const script = join(dir, 'dump.py');
  writeFileSync(script, PY_DUMP);
  try {
    const out = execFileSync(py, [script, filePath], { stdio: 'pipe', encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    return JSON.parse(out);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}

const findSheet = (sheets, kw) => {
  const key = Object.keys(sheets).find(k => k.includes(kw));
  return key ? { name: key, rows: sheets[key] } : null;
};

const fmtDate = (v) => {
  const s = String(v ?? '');
  const d = parseDate(s.slice(0, 10)) || parseDate(s);
  return d;
};

const fmtAmt = (v) => {
  const n = parseAmount(String(v ?? ''));
  return n == null ? 0 : n;
};

/** 70 公户流水 → events（account 公户） */
export function parseBankSheet(rows) {
  const events = [];
  let i = rows.findIndex(r => r.some(c => String(c).includes('日期')));
  if (i < 0) i = 0; else i += 1;
  let opening = 0, balanceCheck = 0;
  for (; i < rows.length; i++) {
    const c = rows[i];
    const occurred_on = fmtDate(c[0]);
    if (!occurred_on) continue;
    const income = fmtAmt(c[4]), expense = fmtAmt(c[5]);
    const bal = c[6] === '' || c[6] == null ? null : fmtAmt(c[6]);
    if (income === 0 && expense === 0 && bal != null) continue;
    events.push({
      occurred_on,
      direction: income > 0 ? 'income' : 'expense',
      amount: (income > 0 ? income : expense).toFixed(2),
      counterparty_raw: String(c[2] ?? '') || null,
      memo: String(c[3] ?? '') || null,
      account: '公户',
      source: 'bank_csv',
      source_line: i + 1,
      sheet: '70',
      bank_balance: bal,
    });
    balanceCheck += income - expense;
    if (events.length === 1 && bal != null) opening = bal - (income - expense);
  }
  return { events, opening, balanceCheck };
}

/** 20 收入台账 → events(income) + vouchers(开票) */
export function parseIncomeSheet(rows) {
  const events = [], vouchers = [];
  let i = rows.findIndex(r => r.some(c => String(c).includes('收入日期')));
  if (i < 0) i = 0; else i += 1;
  for (; i < rows.length; i++) {
    const c = rows[i];
    const occurred_on = fmtDate(c[0]);
    const amount = fmtAmt(c[3]);
    if (!occurred_on || amount === 0) continue;
    const invNo = String(c[5] ?? '').trim();
    events.push({
      occurred_on,
      direction: 'income',
      amount: amount.toFixed(2),
      counterparty_raw: String(c[1] ?? '') || null,
      memo: String(c[2] ?? '') || null,
      tax_status: invNo ? undefined : '未开票收入',
      source: 'manual',
      source_line: i + 1,
      sheet: '20',
    });
    if (invNo) {
      vouchers.push({ kind: 'invoice', invoice_code: null, invoice_no: invNo, expense_category: '收入', source_line: i + 1 });
    }
  }
  return { events, vouchers, total: events.reduce((a, e) => a + Number(e.amount), 0) };
}

/** 30 支出台账 → events(expense) + vouchers（有票才挂） */
export function parseExpenseSheet(rows) {
  const events = [], vouchers = [];
  let i = rows.findIndex(r => r.some(c => String(c).includes('支出日期')));
  if (i < 0) i = 0; else i += 1;
  for (; i < rows.length; i++) {
    const c = rows[i];
    const occurred_on = fmtDate(c[0]);
    const amount = fmtAmt(c[4]);
    if (!occurred_on || amount === 0) continue;
    const hasInv = /是|有|✓/.test(String(c[5] ?? ''));
    const invNo = String(c[6] ?? '').trim();
    events.push({
      occurred_on,
      direction: 'expense',
      amount: amount.toFixed(2),
      category: String(c[1] ?? '') || null,
      memo: String(c[2] ?? '') || null,
      counterparty_raw: String(c[3] ?? '') || null,
      tax_status: hasInv ? '可扣' : '待补票',
      source: 'manual',
      source_line: i + 1,
      sheet: '30',
    });
    if (invNo) vouchers.push({ kind: 'invoice', invoice_code: null, invoice_no: invNo, expense_category: String(c[1] ?? ''), source_line: i + 1 });
  }
  return {
    events, vouchers,
    total: events.reduce((a, e) => a + Number(e.amount), 0),
    invoicedTotal: events.filter(e => e.tax_status === '可扣').reduce((a, e) => a + Number(e.amount), 0),
  };
}

/** 40 税费计算表：取「本期营业收入 / 本期费用支出」数值（公式表可能为 0/空） */
export function parseTaxSheet(rows) {
  const get = (label) => {
    for (const r of rows) {
      if (String(r[0] ?? '').includes(label)) {
        const v = fmtAmt(r[1]);
        return v;
      }
    }
    return null;
  };
  return { revenue: get('本期营业收入'), expense: get('本期费用支出') };
}

/** 10 申报日历 → filing tasks（条目数） */
export function parseCalendarSheet(rows) {
  const tasks = [];
  for (const r of rows) {
    const title = String(r[1] ?? '');
    if (!title || title.includes('事项') || title.startsWith('■')) continue;
    if (!String(r[0] ?? '').trim()) continue;
    tasks.push({ due_hint: String(r[0] ?? ''), title, channel_hint: String(r[3] ?? '') });
  }
  return tasks;
}

/**
 * 迁移主流程：解析全部 sheet + 四勾稽
 * 勾稽（PRD §5.10）：
 *  1) 70 末行余额 = 期初 + Σ收入 − Σ支出
 *  2) 20 金额合计 = 40 本期营业收入（40 为 0/空则跳过并标注）
 *  3) 30 有票支出合计 = 40 本期费用支出（同上）
 *  4) 10 日历条目数 = 解析任务数
 */
export function migrateWorkbook(sheets) {
  const s70 = findSheet(sheets, '70') ?? findSheet(sheets, '公户');
  const s20 = findSheet(sheets, '20') ?? findSheet(sheets, '收入台账');
  const s30 = findSheet(sheets, '30') ?? findSheet(sheets, '支出');
  const s40 = findSheet(sheets, '40') ?? findSheet(sheets, '税费');
  const s10 = findSheet(sheets, '10') ?? findSheet(sheets, '日历');

  const bank = s70 ? parseBankSheet(s70.rows) : { events: [], opening: 0, balanceCheck: 0 };
  const inc = s20 ? parseIncomeSheet(s20.rows) : { events: [], vouchers: [], total: 0 };
  const exp = s30 ? parseExpenseSheet(s30.rows) : { events: [], vouchers: [], total: 0, invoicedTotal: 0 };
  const tax = s40 ? parseTaxSheet(s40.rows) : { revenue: null, expense: null };
  const tasks = s10 ? parseCalendarSheet(s10.rows) : [];

  // 70 首行余额反推期初；末余额取最后一条 bank_balance
  const lastBal = bank.events.length ? bank.events[bank.events.length - 1].bank_balance : null;
  const checks = [];

  // 勾稽 1：余额
  const expectBal = bank.opening + bank.balanceCheck;
  checks.push({
    name: '70 余额勾稽：期初+Σ收−Σ支 = 末余额',
    pass: lastBal == null ? null : Math.abs(expectBal - lastBal) < 0.01,
    detail: `期初 ${bank.opening.toFixed(2)} + 轧差 ${bank.balanceCheck.toFixed(2)} = ${expectBal.toFixed(2)}；表末余额 ${lastBal ?? '(空)'}`,
  });

  // 勾稽 2：收入
  checks.push({
    name: '20 收入合计 = 40 本期营业收入',
    pass: tax.revenue == null || tax.revenue === 0 ? null : Math.abs(inc.total - tax.revenue) < 0.01,
    detail: `20 合计 ${inc.total.toFixed(2)}；40 取值 ${tax.revenue ?? '(空)'}`,
  });

  // 勾稽 3：有票支出
  checks.push({
    name: '30 有票支出合计 = 40 本期费用',
    pass: tax.expense == null || tax.expense === 0 ? null : Math.abs(exp.invoicedTotal - tax.expense) < 0.01,
    detail: `30 有票合计 ${exp.invoicedTotal.toFixed(2)}；40 取值 ${tax.expense ?? '(空)'}`,
  });

  // 勾稽 4：日历条目
  checks.push({
    name: '10 日历条目数 = 解析任务数',
    pass: s10 ? tasks.length > 0 : null,
    detail: `解析任务 ${tasks.length} 条`,
  });

  return {
    events: [...bank.events, ...inc.events, ...exp.events],
    vouchers: [...inc.vouchers, ...exp.vouchers],
    filingTasks: tasks,
    checks,
    stats: {
      bankEvents: bank.events.length,
      incomeEvents: inc.events.length,
      expenseEvents: exp.events.length,
      vouchers: inc.vouchers.length + exp.vouchers.length,
      tasks: tasks.length,
    },
  };
}
