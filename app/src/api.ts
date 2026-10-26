// Tauri invoke 封装
import { invoke } from '@tauri-apps/api/core';

export interface DbStatus {
  ok: boolean;
  db_path: string;
  company: string | null;
  event_count: number;
  voucher_count: number;
  task_pending: number;
}

export interface EventInput {
  occurred_on: string;
  direction: string;
  amount: string;
  counterparty?: string | null;
  memo?: string | null;
  category?: string | null;
  tax_status?: string | null;
  account_name?: string | null;
}

export interface EventRow {
  id: string;
  occurred_on: string;
  direction: string;
  amount: string;
  counterparty: string | null;
  memo: string | null;
  category: string | null;
  tax_status: string | null;
  source: string;
}

export interface TaskRow {
  id: string;
  title: string;
  due_on: string;
  status: string;
  channel: string | null;
}

export interface Dashboard {
  company: string | null;
  event_count: number;
  month_income: string;
  month_expense: string;
  tasks: TaskRow[];
  unmatched: number;
}

export interface CompanyInput {
  name: string;
  tax_no: string;
  legal_person?: string | null;
  register_city?: string | null;
  tax_city?: string | null;
  bank_name?: string | null;
  bank_account?: string | null;
}

export interface UnmatchedEvent {
  id: string;
  occurred_on: string;
  direction: string;
  amount: string;
  counterparty: string | null;
  memo: string | null;
  tax_status: string | null;
}

export interface MatchSuggestion {
  event_id: string;
  voucher_id: string;
  score: number;
  tier: string;
  reason: string;
  event_amount: string;
  voucher_no: string | null;
}

export interface DueTask {
  id: string;
  title: string;
  due_on: string;
  days_left: number;
}

export interface FinMonthRow {
  month: string;
  income: number;
  expense: number;
  profit: number;
}

export interface FinCatRow {
  category: string;
  amount: number;
}

export interface FinAccountRow {
  name: string;
  balance: number;
}

export interface FinReport {
  start: string;
  end: string;
  months: FinMonthRow[];
  income_by_cat: FinCatRow[];
  expense_by_cat: FinCatRow[];
  accounts: FinAccountRow[];
  total_income: number;
  total_expense: number;
  total_profit: number;
  ar: number;
  ap: number;
  note: string;
}

export interface SiHfConfig {
  base_min: number;
  base_max: number;
  pension: number;
  medical: number;
  unemploy: number;
  housing: number;
}

export interface SiHfResult {
  base: number;
  pension: number;
  medical: number;
  unemploy: number;
  housing: number;
  total: number;
  net: number;
}

export interface LicenseStatus {
  machine_code: string;
  plan: string;
  expire_on: string | null;
  licensed: boolean;
  trial_days_left: number;
  message: string;
}

export interface IntegrityItem {
  id: string;
  invoice_no: string | null;
  file_path: string | null;
  ok: boolean;
  reason: string;
}

export interface BackupItem {
  name: string;
  path: string;
  size: number;
  kind: string;
  created_at: string;
}

export interface UpdateCheck {
  configured: boolean;
  current: string;
  latest: string | null;
  has_update: boolean;
  notes: string | null;
  url: string | null;
  message: string;
}

export interface LicenseInfo {
  plan: string;
  expire_on: string | null;
  ai_addon: boolean;
  machine_code: string;
}

export interface WorklogRow {
  id: string;
  job_id: string;
  work_on: string;
  hours: string | null;
  note: string | null;
}

export interface FilingDraftRow {
  id: string;
  period: string;
  tax_type: string;
  amounts: string | null;
  created_on: string;
}

export interface ImportResult {
  file: string;
  ok: boolean;
  duplicate: boolean;
  invoice_no: string | null;
  amount: string | null;
  issued_on: string | null;
  seller: string | null;
  category: string | null;
  message: string;
}

export interface MailConfigView {
  host: string;
  port: number;
  user: string;
  folder: string;
  auto: boolean;
  last_fetch: string;
  has_pass: boolean;
}

export interface FetchReport {
  fetched: number;
  imported: number;
  quarantined: number;
  skipped: number;
  errors: string[];
}

export interface VoucherFull {
  id: string;
  kind: string;
  invoice_code: string | null;
  invoice_no: string | null;
  status: string;
  book_status: string;
  reimburse_status: string;
  amount: string | null;
  issued_on: string | null;
  seller: string | null;
  buyer: string | null;
  expense_category: string | null;
  buyer_ok: boolean;
  file_path: string | null;
  has_file: boolean;
}

export interface ReimbursementRow {
  id: string;
  form_date: string;
  applicant: string | null;
  total: string;
  status: string;
  line_count: number;
}

export const api = {
  initApp: () => invoke<DbStatus>('init_app'),
  dashboard: () => invoke<Dashboard>('dashboard'),
  addEvent: (input: EventInput) => invoke<string>('add_event', { input }),
  listEvents: (limit = 100) => invoke<EventRow[]>('list_events', { limit }),
  listTasks: () => invoke<TaskRow[]>('list_tasks'),
  toggleTask: (id: string) => invoke<void>('toggle_task', { id }),
  getCompany: () => invoke<CompanyInput | null>('get_company'),
  saveCompany: (input: CompanyInput) => invoke<void>('save_company', { input }),
  backupNow: () => invoke<string>('backup_now'),
  importCsv: (path: string) => invoke<number>('import_bank_csv', { path }),
  listUnmatched: () => invoke<UnmatchedEvent[]>('list_unmatched'),
  matchSuggestions: () => invoke<MatchSuggestion[]>('match_suggestions'),
  confirmMatch: (eventId: string, voucherId: string) =>
    invoke<void>('confirm_match', { eventId, voucherId }),
  applyDisposition: (eventId: string, taxStatus: string) =>
    invoke<void>('apply_disposition', { eventId, taxStatus }),
  dueTasks: () => invoke<DueTask[]>('due_tasks'),
  listVouchers: () => invoke<VoucherFull[]>('list_vouchers'),
  addVoucher: (input: {
    invoice_no: string;
    invoice_code?: string | null;
    amount?: string | null;
    issued_on?: string | null;
    seller?: string | null;
    buyer?: string | null;
  }) => invoke<string>('add_voucher', { input }),
  createReimbursement: (voucherIds: string[], applicant?: string | null, memo?: string | null) =>
    invoke<string>('create_reimbursement', { voucherIds, applicant, memo }),
  markReimbursed: (rbId: string) => invoke<void>('mark_reimbursed', { rbId }),
  listReimbursements: () => invoke<ReimbursementRow[]>('list_reimbursements'),
  exportDocument: (name: string, html: string) => invoke<string>('export_document', { name, html }),
  reimbursementLines: (rbId: string) =>
    invoke<{ expense_on: string | null; category: string | null; invoice_no: string | null; amount: string }[]>(
      'reimbursement_lines',
      { rbId },
    ),
  taxReport: (start: string, end: string) => invoke<TaxReport>('tax_report', { start, end }),
  exportTaxDraft: (start: string, end: string) => invoke<string>('export_tax_draft', { start, end }),
  listParties: () => invoke<PartyRow[]>('list_parties'),
  addParty: (input: { name: string; ptype: string; id_card?: string | null; contact?: string | null }) =>
    invoke<string>('add_party', { input }),
  listJobs: () => invoke<JobRow[]>('list_jobs'),
  addJob: (input: { title: string; worker_id?: string | null; start_on?: string | null; end_on?: string | null; total_agreed?: string | null }) =>
    invoke<string>('add_job', { input }),
  jobPay: (jobId: string, amount: string) => invoke<string>('job_pay', { jobId, amount }),
  jobSettle: (jobId: string) => invoke<void>('job_settle', { jobId }),
  addWithhold: (payeeId: string, income: string, period: string) =>
    invoke<string>('add_withhold', { payeeId, income, period }),
  listRules: () => invoke<RuleRow[]>('list_rules'),
  toggleRule: (id: string) => invoke<void>('toggle_rule', { id }),
  applyRules: () => invoke<number>('apply_rules'),
  arAp: () => invoke<ArApRow[]>('ar_ap'),
  settleEvent: (eventId: string) => invoke<void>('settle_event', { eventId }),
  bizBrief: () => invoke<BizBrief>('biz_brief'),
  fetchInvoices: (host: string, port: number, user: string, pass: string, folder?: string) =>
    invoke<FetchReport>('fetch_invoices', { host, port, user, pass, folder }),
  fetchInvoicesSaved: (force?: boolean) => invoke<FetchReport>('fetch_invoices_saved', { force: force ?? false }),
  importVoucherFiles: (paths: string[]) => invoke<ImportResult[]>('import_voucher_files', { paths }),
  pickVoucherFiles: () => invoke<string[]>('pick_voucher_files'),
  mailConfigGet: () => invoke<MailConfigView | null>('mail_config_get'),
  mailConfigSave: (cfg: {
    host: string; port: number; user: string; pass?: string | null; folder?: string | null; auto: boolean;
  }) => invoke<void>('mail_config_save', cfg),
  mailConfigClear: () => invoke<void>('mail_config_clear'),
  openVoucherFile: (id: string) => invoke<string>('open_voucher_file', { id }),
  backupsList: () => invoke<BackupItem[]>('backups_list'),
  backupInfo: () => invoke<[string, BackupItem[]]>('backup_info'),
  verifyVouchers: () => invoke<IntegrityItem[]>('verify_vouchers'),
  openLogsDir: () => invoke<string>('open_logs_dir'),
  licenseStatus: () => invoke<LicenseStatus>('license_status'),
  licenseActivate: (code: string) => invoke<LicenseStatus>('license_activate', { code }),
  exportAllData: () => invoke<string>('export_all_data'),
  pickDbFile: () => invoke<string | null>('pick_db_file'),
  finReport: (start: string, end: string) => invoke<FinReport>('fin_report', { start, end }),
  siHfGet: () => invoke<SiHfConfig>('si_hf_get'),
  siHfSave: (config: SiHfConfig) => invoke<void>('si_hf_save', { config }),
  siHfCalc: (gross: number) => invoke<SiHfResult>('si_hf_calc', { gross }),
  downloadUpdate: () => invoke<string>('download_update'),
  restoreBackup: (path: string) => invoke<string>('restore_backup', { path }),
  restoreEncrypted: (path: string, password: string) =>
    invoke<string>('restore_encrypted', { path, password }),
  updateSourceGet: () => invoke<string>('update_source_get'),
  updateSourceSave: (url: string) => invoke<void>('update_source_save', { url }),
  updateCheck: () => invoke<UpdateCheck>('update_check'),
  licenseInfo: () => invoke<LicenseInfo>('license_info'),
  addWorklog: (jobId: string, workOn: string, hours?: string | null, note?: string | null) =>
    invoke<string>('add_worklog', { jobId, workOn, hours, note }),
  listWorklogs: (jobId: string) => invoke<WorklogRow[]>('list_worklogs', { jobId }),
  listFilingDrafts: (limit?: number) => invoke<FilingDraftRow[]>('list_filing_drafts', { limit }),
  parseExternalDoc: (path: string, applicant?: string | null) =>
    invoke<{ invoice_nos: string[]; matched: number; unmatched: number; hint: string }>('parse_external_doc', {
      path,
      applicant,
    }),
  aiChat: (req: { message: string; persona: string; api_key: string; base_url?: string; model?: string }) =>
    invoke<{ answer: string; context_note: string }>('ai_chat', { req }),
  backupEncrypted: (password: string) => invoke<string>('backup_encrypted', { password }),
  verifyEncrypted: (path: string, password: string) => invoke<boolean>('verify_encrypted', { path, password }),
  statsSeries: () =>
    invoke<{
      months: { month: string; income: number; expense: number }[];
      categories: { category: string; expense: number }[];
      income_categories: { category: string; expense: number }[];
      year_income: number;
      year_expense: number;
      month_income: number;
      month_expense: number;
    }>('stats_series'),
  seedDefaultTasks: () => invoke<number>('seed_default_tasks'),
  exportFilingForms: (start: string, end: string) => invoke<string>('export_filing_forms', { start, end }),
  searchAll: (q: string) =>
    invoke<{ kind: string; id: string; title: string; sub: string; amount: string }[]>('search_all', { q }),
  listEmployees: () =>
    invoke<EmployeeRow[]>('list_employees'),
  addEmployee: (input: { name: string; id_card?: string | null; title?: string | null; hire_on?: string | null; base_salary?: string | null }) =>
    invoke<string>('add_employee', { input }),
  wageTaxPreview: (employeeId: string, periodMonth: string, gross: string, siHf: string) =>
    invoke<number>('wage_tax_preview', { employeeId, periodMonth, gross, siHf }),
  addPayroll: (employeeId: string, periodMonth: string, gross: string, siHf: string, tax: string, net: string) =>
    invoke<string>('add_payroll', { employeeId, periodMonth, gross, siHf, tax, net }),
  listPayroll: () => invoke<PayrollRow[]>('list_payroll'),
  addBonus: (employeeId: string, amount: string, memo: string) =>
    invoke<string>('add_bonus', { employeeId, amount, memo }),
  listAccounts: () => invoke<AccountRow[]>('list_accounts'),
  addAccount: (input: { name: string; atype?: string | null; opening_balance?: string | null; opening_date?: string | null; is_company?: boolean }) =>
    invoke<string>('add_account', { input }),
  accountEvents: (accountId: string, limit = 50) => invoke<EventRow[]>('account_events', { accountId, limit }),
  transfer: (fromId: string, toId: string, amount: string, occurredOn: string, memo?: string | null) =>
    invoke<string>('transfer', { fromId, toId, amount, occurredOn, memo }),
  updateAccount: (id: string, input: { name: string; atype?: string | null; opening_balance?: string | null; opening_date?: string | null; is_company?: boolean }) =>
    invoke<void>('update_account', { id, input }),
  deleteAccount: (id: string) => invoke<void>('delete_account', { id }),
  accountTransfers: (accountId: string) => invoke<EventRow[]>('account_transfers', { accountId }),
  exportText: (name: string, content: string, ext: string) => invoke<string>('export_text', { name, content, ext }),
  updateEvent: (id: string, patch: {
    occurred_on?: string; direction?: string; amount?: string; counterparty?: string | null;
    memo?: string | null; category?: string | null; tax_status?: string | null; account_name?: string;
  }) => invoke<void>('update_event', { id, patch }),
  deleteEvent: (id: string) => invoke<void>('delete_event', { id }),
};

export interface AccountRow {
  id: string;
  name: string;
  atype: string;
  is_company: boolean;
  opening_balance: string;
  opening_date: string | null;
  balance: number;
  event_count: number;
}

export interface EmployeeRow {
  id: string;
  name: string;
  id_card: string | null;
  title: string | null;
  hire_on: string | null;
  base_salary: string | null;
  rb_count: number;
  rb_total: string;
  advance_open: string;
}

export interface PayrollRow {
  id: string;
  period_month: string;
  employee: string;
  gross: string;
  si_hf: string;
  tax: string;
  net: string;
}

export interface TaxReport {
  period: string;
  income_special: number;
  income_general: number;
  income_unbilled: number;
  income_total: number;
  expense_total: number;
  expense_deductible: number;
  expense_nodeduct: number;
  profit: number;
  vat_payable: number;
  vat_detail: string;
  cit_payable: number;
  labor_withheld: number;
  labor_count: number;
  note: string;
}

export interface PartyRow {
  id: string;
  name: string;
  ptype: string;
  id_card: string | null;
  contact: string | null;
  total_paid: string | null;
}

export interface JobRow {
  id: string;
  title: string;
  worker: string | null;
  start_on: string | null;
  end_on: string | null;
  total_agreed: string | null;
  total_paid: string;
  status: string;
}

export interface RuleRow {
  id: string;
  name: string | null;
  conditions: string;
  actions: string;
  auto_apply: number;
  enabled: number;
}

export interface ArApRow {
  party: string;
  direction: string;
  total: number;
  count: number;
  oldest: string;
}

export interface BizBrief {
  income_30: number;
  expense_30: number;
  profit_30: number;
  unsettled_out: number;
  overdue_hint: string;
  cash_balance: number;
  forecast_90: number;
  vat_progress: number;
  vat_left: number;
  top_expense: string;
}
