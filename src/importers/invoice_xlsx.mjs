// 发票清单 xlsx 导入器（税局全量发票导出 / 通用发票表）
// 实现策略：Python openpyxl 桥接（本机已验证可用）——生产版由 Rust 侧解析替换，接口不变
// parseInvoiceXlsx(filePath) → { rows[], warnings[] }
// row: { invoice_code, invoice_no, date, amount, tax, total, buyer, seller, direction, project }

import { execFileSync } from 'node:child_process';
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const PY_SCRIPT = `
import json, sys
from openpyxl import load_workbook
wb = load_workbook(sys.argv[1], read_only=True, data_only=True)
ws = wb.worksheets[0]
rows = []
for r in ws.iter_rows(values_only=True):
    rows.append(["" if v is None else str(v) for v in r])
print(json.dumps(rows, ensure_ascii=False))
`;

function runPython() {
  const candidates = [process.env.MIMO_PYTHON, 'python', 'python3', 'py'].filter(Boolean);
  let lastErr;
  for (const bin of candidates) {
    try {
      execFileSync(bin, ['-c', 'import openpyxl; print("ok")'], { stdio: 'pipe' });
      return bin;
    } catch (e) { lastErr = e; }
  }
  throw new Error(`no python with openpyxl: ${lastErr?.message}`);
}

function guessCol(header) {
  const g = {};
  header.forEach((h, i) => {
    const s = String(h);
    if (/发票代码/.test(s)) g.invoice_code ??= i;
    if (/发票号码|发票号/.test(s)) g.invoice_no ??= i;
    if (/开票日期|日期/.test(s)) g.date ??= i;
    if (/金额(?!税)|不含税/.test(s)) g.amount ??= i;
    if (/税额/.test(s)) g.tax ??= i;
    if (/价税合计|合计金额|总金额/.test(s)) g.total ??= i;
    if (/购方|购买方|付款方/.test(s)) g.buyer ??= i;
    if (/销方|销售方|收款方/.test(s)) g.seller ??= i;
    if (/项目|货物或应税劳务|商品/.test(s)) g.project ??= i;
  });
  return g;
}

export function parseInvoiceRows(matrix, mapping = null) {
  if (!matrix.length) return { rows: [], warnings: ['空表'], mapping: null };
  let map = mapping, start = 0;
  if (!map) {
    map = guessCol(matrix[0]);
    if (map.invoice_no == null) { map = guessCol(['发票代码', '发票号码', '开票日期', '金额', '税额', '购方', '销方']); start = 0; }
    else start = 1;
  }
  const warnings = [];
  const rows = [];
  for (let i = start; i < matrix.length; i++) {
    const c = matrix[i];
    const invoice_no = map.invoice_no != null ? String(c[map.invoice_no] ?? '').trim() : '';
    if (!invoice_no) continue;
    rows.push({
      invoice_code: map.invoice_code != null ? String(c[map.invoice_code] ?? '').trim() || null : null,
      invoice_no,
      date: map.date != null ? String(c[map.date] ?? '').trim().slice(0, 10) : null,
      amount: map.amount != null ? String(c[map.amount] ?? '').trim() : null,
      tax: map.tax != null ? String(c[map.tax] ?? '').trim() : null,
      total: map.total != null ? String(c[map.total] ?? '').trim() : null,
      buyer: map.buyer != null ? String(c[map.buyer] ?? '').trim() : null,
      seller: map.seller != null ? String(c[map.seller] ?? '').trim() : null,
      project: map.project != null ? String(c[map.project] ?? '').trim() : null,
      source_line: i + 1,
    });
  }
  return { rows, warnings, mapping: map };
}

export function parseInvoiceXlsx(filePath) {
  const py = runPython();
  const dir = mkdtempSync(join(tmpdir(), 'zrsj-xlsx-'));
  const script = join(dir, 'dump.py');
  writeFileSync(script, PY_SCRIPT);
  try {
    const out = execFileSync(py, [script, filePath], { stdio: 'pipe', encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });
    const matrix = JSON.parse(out);
    return parseInvoiceRows(matrix);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
}
