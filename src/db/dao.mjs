// DAO：Phase 1 核心表的最小读写（供冒烟与后续 Tauri 命令复用）
import { randomUUID } from 'node:crypto';

const now = () => new Date().toISOString();

export function upsertCompany(db, data) {
  const id = data.id ?? 'main';
  db.prepare(`
    INSERT INTO company(id,name,tax_no,legal_person,address_phone,bank_name,bank_account,
      taxpayer_type,filing_cycle,register_city,tax_city,related_employer,created_at,updated_at)
    VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?)
    ON CONFLICT(id) DO UPDATE SET
      name=excluded.name, tax_no=excluded.tax_no, legal_person=excluded.legal_person,
      address_phone=excluded.address_phone, bank_name=excluded.bank_name,
      bank_account=excluded.bank_account, taxpayer_type=excluded.taxpayer_type,
      filing_cycle=excluded.filing_cycle, register_city=excluded.register_city,
      tax_city=excluded.tax_city, related_employer=excluded.related_employer,
      updated_at=excluded.updated_at
  `).run(id, data.name, data.tax_no, data.legal_person ?? null, data.address_phone ?? null,
    data.bank_name ?? null, data.bank_account ?? null, data.taxpayer_type ?? 'small',
    data.filing_cycle ?? 'quarter', data.register_city ?? null, data.tax_city ?? null,
    data.related_employer ?? null, now(), now());
  return id;
}

export function insertEvent(db, e) {
  const id = e.id ?? randomUUID();
  const t = now();
  db.prepare(`
    INSERT INTO event(id,occurred_on,direction,amount,counterparty_raw,party_id,account_id,
      pair_id,pair_status,memo,category,source,tax_status,reimburse_status,settle_status,
      job_id,reimburse_id,import_batch_id,created_at,updated_at)
    VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
  `).run(id, e.occurred_on, e.direction, String(e.amount), e.counterparty_raw ?? null,
    e.party_id ?? null, e.account_id, e.pair_id ?? null, e.pair_status ?? 'none',
    e.memo ?? null, e.category ?? null, e.source ?? 'manual', e.tax_status ?? null,
    e.reimburse_status ?? 'none', e.settle_status ?? 'none', e.job_id ?? null,
    e.reimburse_id ?? null, e.import_batch_id ?? null, t, t);
  return id;
}

export function insertVoucher(db, v) {
  const id = v.id ?? randomUUID();
  const t = now();
  db.prepare(`
    INSERT INTO voucher(id,kind,invoice_code,invoice_no,status,red_offset_of,
      book_status,reimburse_status,expense_category,file_path,is_original,sha256,
      archive_dir,event_id,source_mail,created_at,updated_at)
    VALUES(?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)
  `).run(id, v.kind, v.invoice_code ?? null, v.invoice_no ?? null, v.status ?? 'normal',
    v.red_offset_of ?? null, v.book_status ?? 'unbooked', v.reimburse_status ?? 'none',
    v.expense_category ?? null, v.file_path ?? null, v.is_original ?? 1, v.sha256 ?? null,
    v.archive_dir ?? null, v.event_id ?? null, v.source_mail ?? null, t, t);
  return id;
}

/** 导入幂等：返回 {batchId, isNew} */
export function beginImport(db, { fileHash, sourceType, fileName, rowCount = 0 }) {
  const existing = db.prepare(`SELECT id FROM import_batch WHERE file_hash=?`).get(fileHash);
  if (existing) return { batchId: existing.id, isNew: false };
  const id = randomUUID();
  db.prepare(`INSERT INTO import_batch(id,file_hash,source_type,file_name,row_count,imported_at)
    VALUES(?,?,?,?,?,?)`).run(id, fileHash, sourceType, fileName ?? null, rowCount, now());
  return { batchId: id, isNew: true };
}

export function insertParty(db, p) {
  const id = p.id ?? randomUUID();
  db.prepare(`INSERT INTO party(id,name,name_norm,type,id_card,contact,created_at)
    VALUES(?,?,?,?,?,?,?)`).run(id, p.name, p.name_norm ?? null, p.type, p.id_card ?? null,
    p.contact ?? null, now());
  return id;
}

export function listEvents(db, { limit = 50 } = {}) {
  return db.prepare(`SELECT * FROM event ORDER BY occurred_on DESC LIMIT ?`).all(limit);
}
