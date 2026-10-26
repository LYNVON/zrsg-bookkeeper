import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, rmSync, mkdtempSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { migrate, openDb } from '../src/db/migrate.mjs';
import { backupDb, verifyBackup } from '../src/db/backup.mjs';
import * as dao from '../src/db/dao.mjs';
import { migrateWorkbook, readWorkbook, parseBankSheet, parseIncomeSheet } from '../src/importers/excel_workbook.mjs';

const REAL_WB = 'C:\\Users\\93783\\Desktop\\自然拾光_财务全套（记账报销报税）.xlsx';

// ── 迁移向导：纯函数层（合成矩阵） ──────────────────────────

test('70 公户流水解析与余额轧差', () => {
  const rows = [
    ['日期', '类型', '对方', '摘要', '收入', '支出', '余额', '备注'],
    ['2026-10-08', '收入', '斐然精准普拉提保利悦公馆店', '广告服务货款', 4600, '', 4600, '示例'],
  ];
  const { events, opening, balanceCheck } = parseBankSheet(rows);
  assert.equal(events.length, 1);
  assert.equal(events[0].direction, 'income');
  assert.equal(opening, 0);
  assert.equal(balanceCheck, 4600);
});

test('20 收入台账：未开票标 tax_status，开票挂凭证', () => {
  const rows = [
    ['收入日期', '客户名称', '项目/事由', '金额', '开票状态', '发票号码', '备注'],
    ['2026-10-08', '斐然', '广告', 4600, '未开票', '', ''],
    ['2026-10-09', '乙公司', '服务', 1000, '已开票', '2644000000001', ''],
  ];
  const { events, vouchers, total } = parseIncomeSheet(rows);
  assert.equal(events.length, 2);
  assert.equal(events[0].tax_status, '未开票收入');
  assert.equal(vouchers.length, 1);
  assert.equal(total, 5600);
});

test('四勾稽（合成工作簿）', () => {
  const sheets = {
    '70 公户流水': [
      ['日期', '类型', '对方', '摘要', '收入', '支出', '余额'],
      ['2026-10-08', '收入', '甲', '货款', 4600, '', 4600],
    ],
    '20 收入台账': [
      ['收入日期', '客户名称', '项目', '金额', '开票状态', '发票号码'],
      ['2026-10-08', '甲', '货款', 4600, '未开票', ''],
    ],
    '30 支出报销台账': [
      ['支出日期', '费用类别', '事由', '对方', '金额', '发票', '发票号码'],
      ['2026-10-09', '办公', '耗材', '丙', 300, '是', '2644000000002'],
    ],
    '40 税费计算表': [
      ['本期营业收入（含未开票）', 4600],
      ['本期费用支出', 300],
    ],
    '10 申报日历': [
      ['时间/期限', '事项', '做什么', '渠道'],
      ['每季度', '增值税申报', '申报', '电子税务局'],
    ],
  };
  const r = migrateWorkbook(sheets);
  assert.equal(r.stats.incomeEvents, 1);
  assert.equal(r.stats.expenseEvents, 1);
  assert.equal(r.checks.length, 4);
  // 40 表合成数据带数值 → 勾稽必须真比对
  const c2 = r.checks.find(c => c.name.includes('20 收入'));
  assert.equal(c2.pass, true);
  const c3 = r.checks.find(c => c.name.includes('30 有票'));
  assert.equal(c3.pass, true);
});

// ── 真实工作簿验收（存在才跑） ─────────────────────────────

test('真实工作簿迁移（示例数据四勾稽）', (t) => {
  if (!existsSync(REAL_WB)) { t.skip('工作簿不在桌面，跳过'); return; }
  const sheets = readWorkbook(REAL_WB);
  const r = migrateWorkbook(sheets);
  console.log('  stats:', JSON.stringify(r.stats));
  for (const c of r.checks) console.log(`  [${c.pass === true ? 'PASS' : c.pass === false ? 'FAIL' : 'SKIP'}] ${c.name} — ${c.detail}`);
  assert.ok(r.stats.bankEvents >= 1, '至少迁入示例流水');
  // 没填数的勾稽允许 SKIP（null），但不允许 FAIL
  assert.ok(r.checks.every(c => c.pass !== false), '勾稽无失败项');
});

// ── 备份导出 ───────────────────────────────────────────────

test('VACUUM INTO 备份 + 校验可读', () => {
  const dir = mkdtempSync(join(tmpdir(), 'zrsj-bk-'));
  const dbPath = join(dir, 'src.db');
  const db = openDb(dbPath);
  migrate(db);
  dao.upsertCompany(db, { name: '测试公司', tax_no: 'T123' });
  db.close();

  const outDir = join(dir, 'backups');
  const { file, size } = backupDb(dbPath, outDir, { tag: 'test' });
  assert.ok(existsSync(file));
  assert.ok(size > 0);
  const v = verifyBackup(file);
  assert.equal(v.ok, true);
  rmSync(dir, { recursive: true, force: true });
});
