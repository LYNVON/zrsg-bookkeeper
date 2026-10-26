-- 自然拾光记账客户端 · 基线 DDL v1
-- 对应 PRD v1.8 §4 数据模型。幂等可重放（IF NOT EXISTS）。
-- 元表

CREATE TABLE IF NOT EXISTS schema_meta (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS change_log (
  id         INTEGER PRIMARY KEY AUTOINCREMENT,
  entity     TEXT NOT NULL,
  entity_id  TEXT NOT NULL,
  action     TEXT NOT NULL,          -- insert|update|delete|merge|status
  before     TEXT,                   -- JSON
  after      TEXT,                   -- JSON
  at         TEXT NOT NULL           -- ISO8601
);

CREATE TABLE IF NOT EXISTS import_batch (
  id          TEXT PRIMARY KEY,      -- uuid
  file_hash   TEXT NOT NULL,         -- sha256，幂等键
  source_type TEXT NOT NULL,         -- bank_csv|invoice_xlsx|invoice_pdf|invoice_xml|excel_workbook
  file_name   TEXT,
  row_count   INTEGER DEFAULT 0,
  imported_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_import_batch_hash ON import_batch(file_hash);

-- 公司档案（全局唯一一行）

CREATE TABLE IF NOT EXISTS company (
  id              TEXT PRIMARY KEY,
  name            TEXT NOT NULL,
  tax_no          TEXT NOT NULL,
  legal_person    TEXT,
  address_phone   TEXT,
  bank_name       TEXT,
  bank_account    TEXT,
  taxpayer_type   TEXT NOT NULL DEFAULT 'small',   -- small|general
  filing_cycle    TEXT NOT NULL DEFAULT 'quarter', -- quarter|month
  register_city   TEXT,
  tax_city        TEXT,
  related_employer TEXT,
  created_at      TEXT NOT NULL,
  updated_at      TEXT NOT NULL
);

-- 账户（公户/微信/支付宝/个人卡/现金）

CREATE TABLE IF NOT EXISTS account (
  id             TEXT PRIMARY KEY,
  name           TEXT NOT NULL,
  type           TEXT NOT NULL,       -- bank|wechat|alipay|personal_card|cash
  is_company     INTEGER NOT NULL DEFAULT 1,
  opening_balance TEXT NOT NULL DEFAULT '0',
  opening_date   TEXT,
  created_at     TEXT NOT NULL
);

-- 对手方（公司/个人；个人身份证全局唯一）

CREATE TABLE IF NOT EXISTS party (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  name_norm   TEXT,
  type        TEXT NOT NULL,          -- company|person
  id_card     TEXT,
  contact     TEXT,
  created_at  TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_party_idcard
  ON party(id_card) WHERE id_card IS NOT NULL;

-- 事项（核心：一件事一条记录）

CREATE TABLE IF NOT EXISTS event (
  id               TEXT PRIMARY KEY,
  occurred_on      TEXT NOT NULL,
  direction        TEXT NOT NULL,     -- income|expense|transfer|refund|loan_in|loan_out|red_offset
  amount           TEXT NOT NULL,     -- 分/元以字符串存 decimal，避免浮点
  counterparty_raw TEXT,
  party_id         TEXT REFERENCES party(id),
  account_id       TEXT NOT NULL REFERENCES account(id),
  pair_id          TEXT,              -- 转账/借款对等配对（仅此类）
  pair_status      TEXT NOT NULL DEFAULT 'none', -- none|half|paired
  memo             TEXT,
  category         TEXT,
  source           TEXT NOT NULL,     -- manual|bank_csv|wechat|alipay|invoice|reimburse|import_dup（多值逗号分隔）
  tax_status       TEXT,              -- 可扣|收据可扣|待补票|不可扣|未开票收入|不涉税
  reimburse_status TEXT NOT NULL DEFAULT 'none', -- none|in_reimburse|reimbursed
  settle_status    TEXT NOT NULL DEFAULT 'none', -- none|unsettled|settled|partial
  job_id           TEXT REFERENCES service_job(id),
  reimburse_id     TEXT REFERENCES reimbursement(id),
  import_batch_id  TEXT REFERENCES import_batch(id),
  created_at       TEXT NOT NULL,
  updated_at       TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS ix_event_date ON event(occurred_on);
CREATE INDEX IF NOT EXISTS ix_event_account ON event(account_id);
CREATE INDEX IF NOT EXISTS ix_event_settle ON event(settle_status);
CREATE INDEX IF NOT EXISTS ix_event_pair ON event(pair_id);

-- 凭证/附件（发票、收据、回单、截图）

CREATE TABLE IF NOT EXISTS voucher (
  id                TEXT PRIMARY KEY,
  kind              TEXT NOT NULL,    -- invoice|receipt|reimburse|slip|none
  invoice_code      TEXT,             -- 传统票代码；数电票为空
  invoice_no        TEXT,             -- 数电票 20 位
  status            TEXT NOT NULL DEFAULT 'normal', -- normal|void|red_offsetted|red_offset_src
  red_offset_of     TEXT REFERENCES voucher(id),
  book_status       TEXT NOT NULL DEFAULT 'unbooked',   -- unbooked|booked
  reimburse_status  TEXT NOT NULL DEFAULT 'none',       -- none|in_reimburse|reimbursed
  expense_category  TEXT,
  file_path         TEXT,
  amount            TEXT,             -- 价税合计（v1.9：报销合计与匹配用）
  issued_on         TEXT,             -- 开票日期
  seller            TEXT,             -- 销方名称
  buyer             TEXT,             -- 购方名称（收票筛查键）
  is_original       INTEGER NOT NULL DEFAULT 1,
  sha256            TEXT,
  archive_dir       TEXT,
  event_id          TEXT REFERENCES event(id),
  source_mail       TEXT,
  created_at        TEXT NOT NULL,
  updated_at        TEXT NOT NULL
);
-- 发票唯一键：规范化键（SQLite 唯一索引中 NULL 互不相等，必须用表达式拼接，数电票 code 为空也能拦重复）
CREATE UNIQUE INDEX IF NOT EXISTS ux_voucher_invoice
  ON voucher(COALESCE(invoice_code,'') || '|' || COALESCE(invoice_no,''))
  WHERE invoice_no IS NOT NULL;
CREATE INDEX IF NOT EXISTS ix_voucher_sha ON voucher(sha256);

-- 报销单 + 明细

CREATE TABLE IF NOT EXISTS reimbursement (
  id          TEXT PRIMARY KEY,
  form_date   TEXT NOT NULL,
  applicant   TEXT,
  period      TEXT,
  total       TEXT NOT NULL DEFAULT '0',
  status      TEXT NOT NULL DEFAULT 'draft', -- draft|submitted|paid|void(仅 pre-paid)
  source      TEXT,                          -- manual|excel|external_doc
  import_batch_id TEXT REFERENCES import_batch(id),
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS reimbursement_line (
  id            TEXT PRIMARY KEY,
  reimburse_id  TEXT NOT NULL REFERENCES reimbursement(id),
  expense_on    TEXT,
  category      TEXT,
  memo          TEXT,
  invoice_no    TEXT,
  invoice_code  TEXT,
  amount        TEXT NOT NULL DEFAULT '0',
  event_id      TEXT REFERENCES event(id)   -- 关联被修正的垫付事项
);
CREATE INDEX IF NOT EXISTS ix_rbline_parent ON reimbursement_line(reimburse_id);

-- 劳务工单 + 工时

CREATE TABLE IF NOT EXISTS service_job (
  id            TEXT PRIMARY KEY,
  title         TEXT NOT NULL,
  worker_id     TEXT REFERENCES party(id),
  start_on      TEXT,
  end_on        TEXT,
  total_agreed  TEXT,
  total_paid    TEXT NOT NULL DEFAULT '0',
  status        TEXT NOT NULL DEFAULT 'open', -- open|settled|void
  created_at    TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worklog (
  id       TEXT PRIMARY KEY,
  job_id   TEXT NOT NULL REFERENCES service_job(id),
  work_on  TEXT NOT NULL,
  hours    TEXT,
  note     TEXT
);

-- 工资 + 个税扣缴台账（互斥：同人同期单点落账）

CREATE TABLE IF NOT EXISTS payroll (
  id          TEXT PRIMARY KEY,
  period_month TEXT NOT NULL,          -- YYYY-MM
  employee_id  TEXT NOT NULL REFERENCES party(id),
  gross        TEXT NOT NULL DEFAULT '0',
  si_hf        TEXT NOT NULL DEFAULT '0',
  tax          TEXT NOT NULL DEFAULT '0',
  net          TEXT NOT NULL DEFAULT '0',
  dual_salary_warn INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_payroll_person_period
  ON payroll(employee_id, period_month);

CREATE TABLE IF NOT EXISTS withhold_ledger (
  id        TEXT PRIMARY KEY,
  paid_on   TEXT NOT NULL,
  payee_id  TEXT NOT NULL REFERENCES party(id),
  kind      TEXT NOT NULL,             -- wage|labor
  income    TEXT NOT NULL DEFAULT '0',
  withheld  TEXT NOT NULL DEFAULT '0',
  period    TEXT
);
CREATE UNIQUE INDEX IF NOT EXISTS ux_withhold_person_period
  ON withhold_ledger(payee_id, kind, period);

-- 申报任务 + 底稿

CREATE TABLE IF NOT EXISTS filing_task (
  id          TEXT PRIMARY KEY,
  title       TEXT NOT NULL,
  due_on      TEXT NOT NULL,
  rule_expr   TEXT,                    -- JSON 结构化周期规则
  status      TEXT NOT NULL DEFAULT 'pending', -- pending|done|skipped
  checklist   TEXT,                    -- JSON
  channel     TEXT,                    -- JSON：办理渠道+入口（按城市适配）
  done_at     TEXT
);

CREATE TABLE IF NOT EXISTS filing_draft (
  id       TEXT PRIMARY KEY,
  period   TEXT NOT NULL,
  tax_type TEXT NOT NULL,              -- vat|cit|surcharge|withhold
  amounts  TEXT,                       -- JSON 分口径
  calculation_trace TEXT,              -- JSON 过程留痕
  created_on TEXT NOT NULL
);

-- 规则（结构化条件）

CREATE TABLE IF NOT EXISTS rule (
  id         TEXT PRIMARY KEY,
  name       TEXT,
  conditions TEXT NOT NULL,            -- JSON [{field,op,value}] ≤5 默认可扩展
  actions    TEXT NOT NULL,            -- JSON {category,account,auto_apply}
  priority   INTEGER NOT NULL DEFAULT 100,
  auto_apply INTEGER NOT NULL DEFAULT 0,
  enabled    INTEGER NOT NULL DEFAULT 1
);

-- 授权（账号级）

CREATE TABLE IF NOT EXISTS license (
  id          TEXT PRIMARY KEY,
  account_id  TEXT NOT NULL,
  plan        TEXT NOT NULL DEFAULT 'trial', -- trial|monthly|yearly|permanent
  ai_addon    INTEGER NOT NULL DEFAULT 0,
  expire_on   TEXT,
  local_token TEXT,                    -- JSON：服务器时间戳+硬件指纹锚点
  created_at  TEXT NOT NULL,
  updated_at  TEXT NOT NULL
);
