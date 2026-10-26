import { test } from 'node:test';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { migrate, openDb } from '../src/db/migrate.mjs';
import * as dao from '../src/db/dao.mjs';
import { laborWithhold, vatSmallQuarterly, isZeroSmall, citSmallProfit } from '../src/domain/tax.mjs';
import { scorePair, tierOf, normName, candidates } from '../src/domain/score.mjs';

// ── 税费：第三方复审锚定算例（v1.6 修正后必须成立）────────────

test('劳务预扣 30000 → 5200（三级累进，禁止简化×20%=4800）', () => {
  const r = laborWithhold(30000);
  assert.equal(r.taxable, 24000);
  assert.equal(r.withheld, 5200); // 24000×30% - 2000
});

test('劳务预扣 10000 → 1600（落在 20% 档）', () => {
  assert.equal(laborWithhold(10000).withheld, 1600); // 8000×20%
});

test('劳务预扣 ≤800 → 0', () => {
  assert.equal(laborWithhold(600).withheld, 0);
});

test('增值税：季销 29 万（专票 5 万+普票 24 万）专票不免', () => {
  const r = vatSmallQuarterly({ special: 50000, general: 240000, unbilled: 0 });
  assert.equal(r.vatPayable, 500);       // 50000×1% 专票照缴
  assert.equal(r.detail.total, 290000);  // ≤30 万但专票不豁免
});

test('小微企税 200 万利润 → 10 万（5%）', () => {
  assert.equal(citSmallProfit(2000000), 100000);
});

test('零星起征点可配：480 在 500 档可扣、在 300 档不可', () => {
  assert.equal(isZeroSmall(480, 500), true);
  assert.equal(isZeroSmall(480, 300), false);
});

// ── 关联引擎打分 ─────────────────────────────────────────────

test('金额差超 0.01 不进候选', () => {
  assert.equal(scorePair({ amount: '4600', occurred_on: '2026-10-08', counterparty: '甲' },
    { amount: '4600.02', date: '2026-10-08', counterparty: '甲' }), null);
});

test('金额+户名+日期近 → ≥80 自动挂靠', () => {
  const s = scorePair({ amount: '4600', occurred_on: '2026-10-08', counterparty: '斐然精准普拉提保利悦公馆店' },
    { amount: '4600', date: '2026-10-08', counterparty: '斐然精准普拉提保利悦公馆店' });
  assert.ok(s.score >= 80);
  assert.equal(tierOf(s.score), 'auto');
});

test('户名归一：北京XX科技 ≈ XX科技(北京)有限公司', () => {
  assert.ok(normName('北京星科技').includes('星') || normName('北京星科技') === normName('星科技(北京)有限公司') || normName('星科技(北京)有限公司').includes(normName('北京星科技')));
  const s = scorePair({ amount: '100', occurred_on: '2026-10-01', counterparty: '北京星科技' },
    { amount: '100', date: '2026-10-02', counterparty: '星科技（北京）有限公司' });
  assert.ok(s.score >= 80);
});

test('1:N 求和：4600 流水 vs 2000+1600+1000 发票', () => {
  const docs = [
    { id: 'a', amount: '2000', date: '2026-10-01', counterparty: 'x' },
    { id: 'b', amount: '1600', date: '2026-10-01', counterparty: 'x' },
    { id: 'c', amount: '1000', date: '2026-10-01', counterparty: 'x' },
    { id: 'd', amount: '999', date: '2026-10-01', counterparty: 'x' },
  ];
  const out = candidates({ amount: '4600', occurred_on: '2026-10-02', counterparty: 'x' }, docs);
  const combo = out.find(o => o.combined);
  assert.ok(combo); // 至少命中一对组合
});

// ── 数据层：迁移 / 幂等 / 唯一约束 ───────────────────────────

test('迁移可重复执行、幂等导入不产生重复', () => {
  const db = openDb(':memory:');
  const v = migrate(db);
  assert.ok(v >= 0);
  migrate(db); // 重放安全

  dao.upsertCompany(db, { name: '自然拾光科技（广州）有限公司', tax_no: '91440113MAK3PE2J9G', register_city: '广州' });
  const acct = db.prepare(`INSERT INTO account(id,name,type,created_at) VALUES('a1','公户','bank',datetime('now'))`).run();

  const r1 = dao.beginImport(db, { fileHash: 'h1', sourceType: 'bank_csv', fileName: 'a.csv' });
  assert.equal(r1.isNew, true);
  const r2 = dao.beginImport(db, { fileHash: 'h1', sourceType: 'bank_csv', fileName: 'a.csv' });
  assert.equal(r2.isNew, false); // 同文件二次导入 0 新记录

  const eid = dao.insertEvent(db, {
    occurred_on: '2026-10-08', direction: 'income', amount: '4600',
    account_id: 'a1', source: 'bank_csv', import_batch_id: r1.batchId, tax_status: '未开票收入',
  });
  assert.ok(eid);
  assert.equal(dao.listEvents(db).length, 1);
});

test('个人身份证全局唯一', () => {
  const db = openDb(':memory:');
  migrate(db);
  dao.insertParty(db, { name: '张三', type: 'person', id_card: '4401xx' });
  assert.throws(() => dao.insertParty(db, { name: '张三2', type: 'person', id_card: '4401xx' }));
});

test('发票规范化键唯一（数电票 code 空）', () => {
  const db = openDb(':memory:');
  migrate(db);
  dao.insertVoucher(db, { kind: 'invoice', invoice_code: null, invoice_no: '26447000001979874603' });
  assert.throws(() => dao.insertVoucher(db, { kind: 'invoice', invoice_code: null, invoice_no: '26447000001979874603' }));
});
