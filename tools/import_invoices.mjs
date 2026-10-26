// 发票清单导入 CLI：node tools/import_invoices.mjs <xlsx>
// 复用 invoice_xlsx 解析 → 写入 voucher 表（发票号唯一键去重）
import { existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { migrate, openDb } from '../src/db/migrate.mjs';
import { parseInvoiceXlsx } from '../src/importers/invoice_xlsx.mjs';

const file = resolve(process.argv[2] ?? '');
if (!file || !existsSync(file)) {
  console.error('用法: node tools/import_invoices.mjs <发票清单.xlsx>');
  process.exit(1);
}

const db = openDb(process.env.ZRSJ_DB ?? 'data/zrsj.db');
migrate(db);

const { rows, warnings } = parseInvoiceXlsx(file);
let n = 0, skip = 0;
for (const r of rows) {
  try {
    db.prepare(
      `INSERT INTO voucher(id,kind,invoice_code,invoice_no,status,book_status,reimburse_status,
        expense_category,amount,issued_on,seller,buyer,is_original,created_at,updated_at)
       VALUES(?1,'invoice',?2,?3,'normal','unbooked','none',NULL,?4,?5,?6,?7,1,datetime('now'),datetime('now'))`
    ).run(
      `v-${Math.random().toString(36).slice(2, 10)}`,
      r.invoice_code || null,
      r.invoice_no,
      r.total || r.amount || null,
      r.date || null,
      r.seller || null,
      r.buyer || null,
    );
    n++;
  } catch {
    skip++; // 发票号重复等
  }
}
console.log(`导入完成：新票 ${n} · 跳过（重复等）${skip}`);
for (const w of warnings.slice(0, 5)) console.log('警告:', w);
