// 迁移执行器：schema_meta(version) + db/migrations/NNN_*.sql 顺序执行，事务化
import { DatabaseSync } from 'node:sqlite';
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = join(__dirname, '..', '..');

export function openDb(path) {
  const db = new DatabaseSync(path);
  db.exec('PRAGMA journal_mode = WAL');
  db.exec('PRAGMA foreign_keys = ON');
  return db;
}

export function migrate(db, { baseline = join(ROOT, 'db', 'schema.sql') } = {}) {
  db.exec(readFileSync(baseline, 'utf8'));
  db.exec(`CREATE TABLE IF NOT EXISTS schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL)`);
  const cur = db.prepare(`SELECT value FROM schema_meta WHERE key='version'`).get();
  let version = cur ? Number(cur.value) : 0;

  const migDir = join(ROOT, 'db', 'migrations');
  const files = readdirSync(migDir).filter(f => /^\d+.*\.sql$/.test(f)).sort();
  for (const f of files) {
    const n = Number(f.split('_')[0]);
    if (n <= version) continue;
    db.exec('BEGIN');
    try {
      db.exec(readFileSync(join(migDir, f), 'utf8'));
      db.prepare(`INSERT INTO schema_meta(key,value) VALUES('version',?) ON CONFLICT(key) DO UPDATE SET value=excluded.value`).run(String(n));
      db.exec('COMMIT');
      version = n;
    } catch (e) {
      db.exec('ROLLBACK');
      throw new Error(`migration ${f} failed: ${e.message}`);
    }
  }
  return version;
}
