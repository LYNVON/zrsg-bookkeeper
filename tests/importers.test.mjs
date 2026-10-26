import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { parseBankCsv, parseDate, parseAmount, guessMapping, splitCsvLine } from '../src/importers/bank_csv.mjs';
import { parseInvoiceRows, parseInvoiceXlsx } from '../src/importers/invoice_xlsx.mjs';

// ── 日期/金额解析 ──────────────────────────────────────────

test('日期多格式：2026-10-08 / 2026/10/8 / 20261008 / 2026年10月8日', () => {
  assert.equal(parseDate('2026-10-08'), '2026-10-08');
  assert.equal(parseDate('2026/10/8'), '2026-10-08');
  assert.equal(parseDate('20261008'), '2026-10-08');
  assert.equal(parseDate('2026年10月8日'), '2026-10-08');
});

test('金额清洗：¥1,234.56 / 4600 元 / 空', () => {
  assert.equal(parseAmount('¥1,234.56'), 1234.56);
  assert.equal(parseAmount('4600 元'), 4600);
  assert.equal(parseAmount('-'), null);
});

// ── 银行流水 CSV ───────────────────────────────────────────

const CSV = `交易日期,对方户名,摘要,收入,支出
2026-10-08,斐然精准普拉提保利悦公馆店,广告服务货款,4600,
2026-10-09,延长壳牌(广东)石油有限公司,加油,-,323.45
2026-10-10,张三,临时工人工费,,300
badrow,xx,yy,1,`;

test('CSV 自动猜映射 + 解析 3 行、跳过坏行', () => {
  const { rows, warnings, mapping } = parseBankCsv(CSV);
  assert.equal(rows.length, 3);
  assert.equal(rows[0].occurred_on, '2026-10-08');
  assert.equal(rows[0].direction, 'income');
  assert.equal(rows[0].amount, '4600.00');
  assert.equal(rows[1].direction, 'expense');
  assert.equal(rows[1].amount, '323.45');
  assert.ok(warnings.some(w => w.includes('跳过')));
  assert.ok(mapping.date != null && mapping.income != null);
});

test('单列金额负数判支出', () => {
  const csv = `日期,金额,对方\n2026-10-01,-500,甲\n`;
  const { rows } = parseBankCsv(csv);
  assert.equal(rows[0].direction, 'expense');
  assert.equal(rows[0].amount, '500.00');
});

test('引号字段含逗号不切坏', () => {
  const cells = splitCsvLine('2026-10-01,"甲,乙公司",ok,1,');
  assert.equal(cells[1], '甲,乙公司');
});

// ── 发票 xlsx ─────────────────────────────────────────────

test('发票矩阵解析（列启发式）', () => {
  const matrix = [
    ['发票代码', '发票号码', '开票日期', '金额', '税额', '价税合计', '购方名称', '销方名称', '项目名称'],
    ['044001900111', '26447000001979874603', '2026-10-10', '286.24', '37.21', '323.45', '自然拾光科技（广州）有限公司', '延长壳牌(广东)石油有限公司', '*汽油*车用汽油'],
    ['', '2612000000429188486', '2026-10-11', '326.00', '0', '326.00', '自然拾光科技（广州）有限公司', '北京春田知勤科技有限公司', '*生活服务*信息服务费'],
  ];
  const { rows } = parseInvoiceRows(matrix);
  assert.equal(rows.length, 2);
  assert.equal(rows[0].invoice_no, '26447000001979874603');
  assert.equal(rows[0].invoice_code, '044001900111');
  assert.equal(rows[1].invoice_code, null); // 数电票无代码
  assert.ok(rows[0].buyer.includes('自然拾光'));
});

test('真实 xlsx 往返（python openpyxl 桥接）', (t) => {
  let py = null;
  try {
    execFileSync(process.env.MIMO_PYTHON || 'python', ['-c', 'import openpyxl'], { stdio: 'pipe' });
    py = true;
  } catch { py = false; }
  if (!py) { t.skip('python+openpyxl 不可用'); return; }

  const dir = mkdtempSync(join(tmpdir(), 'zrsj-inv-'));
  const xlsx = join(dir, 'inv.xlsx');
  execFileSync(process.env.MIMO_PYTHON || 'python', ['-c', `
from openpyxl import Workbook
wb = Workbook(); ws = wb.active
ws.append(['发票代码','发票号码','开票日期','金额','税额','价税合计','购方名称','销方名称','项目名称'])
ws.append(['','26447000001979874603','2026-10-10',286.24,37.21,323.45,'自然拾光科技（广州）有限公司','延长壳牌','*汽油*'])
wb.save(r'${xlsx.replace(/\\/g, '\\\\')}')
`], { stdio: 'pipe' });
  try {
    const { rows } = parseInvoiceXlsx(xlsx);
    assert.equal(rows.length, 1);
    assert.equal(rows[0].invoice_no, '26447000001979874603');
    assert.equal(rows[0].invoice_code, null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
