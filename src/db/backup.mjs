// 备份导出（PRD §5.9）：VACUUM INTO 快照 + 附件目录打包说明
// backupDb(dbPath, backupDir) → { file, size }
import { DatabaseSync } from 'node:sqlite';
import { mkdirSync, existsSync, statSync, cpSync } from 'node:fs';
import { join } from 'node:path';

export function backupDb(dbPath, backupDir, { attachmentsDir = null, tag = null } = {}) {
  if (!existsSync(dbPath)) throw new Error(`db not found: ${dbPath}`);
  mkdirSync(backupDir, { recursive: true });
  const stamp = (tag ?? new Date().toISOString().replace(/[:.]/g, '-'));
  const out = join(backupDir, `zrsj-${stamp}.db`);

  const db = new DatabaseSync(dbPath);
  try {
    // online 备份语义：VACUUM INTO 一致性快照，不阻塞写入
    db.exec(`VACUUM INTO '${out.replace(/'/g, "''")}'`);
  } finally {
    db.close();
  }

  let attachments = null;
  if (attachmentsDir && existsSync(attachmentsDir)) {
    attachments = join(backupDir, `zrsj-${stamp}-attachments`);
    cpSync(attachmentsDir, attachments, { recursive: true });
  }

  return { file: out, size: statSync(out).size, attachments };
}

/** 导入恢复前置校验：能打开 + 有 schema_meta */
export function verifyBackup(file) {
  const db = new DatabaseSync(file, { readOnly: true });
  try {
    const t = db.prepare(`SELECT name FROM sqlite_master WHERE type='table' AND name='schema_meta'`).get();
    const ver = t ? db.prepare(`SELECT value FROM schema_meta WHERE key='version'`).get() : null;
    return { ok: !!t, version: ver?.value ?? null };
  } finally {
    db.close();
  }
}
