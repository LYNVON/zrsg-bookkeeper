// Excel 工作簿迁移 CLI：node tools/migrate_workbook.mjs [工作簿路径]
// 解析 → 写入 data/zrsj.db → 打印四勾稽结果
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { migrate, openDb } from '../src/db/migrate.mjs';
import * as dao from '../src/db/dao.mjs';
import { readWorkbook, migrateWorkbook } from '../src/importers/excel_workbook.mjs';

const DEFAULT_WB = 'C:\\Users\\93783\\Desktop\\自然拾光_财务全套（记账报销报税）.xlsx';
const wbPath = resolve(process.argv[2] ?? DEFAULT_WB);
const dbPath = process.env.ZRSJ_DB ?? 'data/zrsj.db';

if (!existsSync(wbPath)) {
  console.error('工作簿不存在:', wbPath);
  process.exit(1);
}

console.log('读取工作簿:', wbPath);
const sheets = readWorkbook(wbPath);
const r = migrateWorkbook(sheets);

console.log(`\n解析结果: 流水/收入/支出事项 ${r.events.length} · 凭证 ${r.vouchers.length} · 申报任务 ${r.filingTasks.length}`);
console.log('\n四勾稽:');
for (const c of r.checks) {
  const mark = c.pass === true ? 'PASS' : c.pass === false ? 'FAIL' : 'SKIP';
  console.log(`  [${mark}] ${c.name} — ${c.detail}`);
}
if (r.checks.some((c) => c.pass === false)) {
  console.error('\n勾稽存在失败项，已中止写入（PRD §5.10：不平阻断）');
  process.exit(2);
}

const db = openDb(dbPath);
migrate(db);

const batch = dao.beginImport(db, {
  fileHash: `wb:${wbPath}`,
  sourceType: 'excel_workbook',
  fileName: wbPath.split(/[\\/]/).pop(),
  rowCount: r.events.length,
});
if (!batch.isNew) {
  console.log('\n该工作簿此前已迁移过（幂等），未重复写入。如需重导请删除 data/zrsj.db 后重试。');
  process.exit(0);
}

dao.upsertCompany(db, {
  name: '自然拾光科技（广州）有限公司',
  tax_no: '91440113MAK3PE2J9G',
  register_city: '广州',
  tax_city: '广州',
});

const acct = db.prepare(`SELECT id FROM account WHERE name='公户'`).get();
if (!acct) {
  db.prepare(`INSERT INTO account(id,name,type,is_company,opening_balance,created_at) VALUES('acct-gonghu','公户','bank',1,'0',datetime('now'))`).run();
}
const acctId = db.prepare(`SELECT id FROM account WHERE name='公户'`).get().id;

let n = 0;
for (const e of r.events) {
  dao.insertEvent(db, {
    ...e,
    account_id: acctId,
    import_batch_id: batch.batchId,
    source: e.source ?? 'manual',
  });
  n++;
}
for (const v of r.vouchers) {
  dao.insertVoucher(db, v);
}
for (const t of r.filingTasks) {
  db.prepare(`INSERT INTO filing_task(id,title,due_on,status,channel,rule_expr,checklist) VALUES(?,?,?, 'pending', ?, NULL, NULL)`).run(
    `ft-${Math.random().toString(36).slice(2, 10)}`,
    t.title,
    '2026-12-31',
    t.channel_hint || null,
  );
}
console.log(`\n已写入 ${dbPath}: 事项 ${n} · 凭证 ${r.vouchers.length} · 申报任务 ${r.filingTasks.length}`);
