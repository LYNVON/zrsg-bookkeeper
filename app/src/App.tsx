import { useEffect, useRef, useState } from 'react';
import { sendNotification } from '@tauri-apps/plugin-notification';
import { api, type BizBrief, type Dashboard, type DueTask, type TaskRow } from './api';
import { BarChart, PieChart, TrendChart } from './charts';
import { Orb, type Persona } from './orb';
import { AssistantDrawer, AccountPage, DocsPage, EmployeePage, FinReportPage, JobsPage, SettingsPage, TaxPage, VaultPage, WorkPage } from './pages';
import logoGold from './assets/logo-gold.png';
import './App.css';

type Tab = 'home' | 'work' | 'accounts' | 'vault' | 'jobs' | 'tax' | 'report' | 'docs' | 'settings';
type VaultSub = 'vault' | 'rb' | 'intake';
type LaborSub = 'temp' | 'emp';

const NAV: { k: Tab; label: string; icon: string; children?: { k: string; label: string }[] }[] = [
  { k: 'home', label: '首页', icon: 'home' },
  { k: 'work', label: '工作事项', icon: 'list' },
  { k: 'accounts', label: '账户', icon: 'wallet' },
  {
    k: 'vault', label: '票夹', icon: 'ticket',
    children: [
      { k: 'vault', label: '发票' },
      { k: 'rb', label: '报销单' },
      { k: 'intake', label: '收票工具' },
    ],
  },
  {
    k: 'jobs', label: '劳务用工', icon: 'case',
    children: [
      { k: 'temp', label: '临时用工' },
      { k: 'emp', label: '正式员工' },
    ],
  },
  { k: 'tax', label: '税务申报', icon: 'doc' },
  { k: 'report', label: '经营报表', icon: 'doc' },
  { k: 'docs', label: '文书', icon: 'file' },
];

function Icon({ name, size = 18 }: { name: string; size?: number }) {
  const paths: Record<string, string> = {
    home: 'M3 10.5 12 3l9 7.5V20a1 1 0 0 1-1 1h-5v-6h-6v6H4a1 1 0 0 1-1-1z',
    list: 'M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01',
    ticket: 'M4 6h16a1 1 0 0 1 1 1v3a2 2 0 0 0 0 4v3a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1v-3a2 2 0 0 0 0-4V7a1 1 0 0 1 1-1z',
    case: 'M8 7V5a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2m-8 0H5a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2V9a2 2 0 0 0-2-2h-5m-8 0h8',
    doc: 'M8 3h8l4 4v14a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zm8 0v4h4M9 13h6M9 17h6',
    file: 'M7 3h7l5 5v13a1 1 0 0 1-1 1H7a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1zm7 0v5h5',
    gear: 'M12 15.5a3.5 3.5 0 1 0 0-7 3.5 3.5 0 0 0 0 7zm8-3.5a8 8 0 0 0-.15-1.4l2-1.55-2-3.45-2.35.95a8 8 0 0 0-2.42-1.4L14.5 2h-4l-.58 2.6a8 8 0 0 0-2.42 1.4L5.15 5.05l-2 3.45 2 1.55A8 8 0 0 0 5 12c0 .48.05.95.15 1.4l-2 1.55 2 3.45 2.35-.95a8 8 0 0 0 2.42 1.4L10.5 22h4l.58-2.6a8 8 0 0 0 2.42-1.4l2.35.95 2-3.45-2-1.55c.1-.45.15-.92.15-1.4z',
    wallet: 'M3 7a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zm14 6a1 1 0 1 0 0 2h3a1 1 0 1 0 0-2z',
    search: 'M11 18a7 7 0 1 0 0-14 7 7 0 0 0 0 14zm10 2-4.35-4.35',
  };
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round">
      <path d={paths[name] ?? paths.file} />
    </svg>
  );
}

export default function App() {
  const [tab, setTab] = useState<Tab>('home');
  const [dash, setDash] = useState<Dashboard | null>(null);
  const [brief, setBrief] = useState<BizBrief | null>(null);
  const [series, setSeries] = useState<Awaited<ReturnType<typeof api.statsSeries>> | null>(null);
  const [tasks, setTasks] = useState<TaskRow[]>([]);
  const [due, setDue] = useState<DueTask[]>([]);
  const [toast, setToast] = useState<string | null>(null);
  const [refreshKey, setRefreshKey] = useState(0);
  const [range, setRange] = useState<'month' | 'year'>('month');
  const [persona, setPersona] = useState<Persona>((localStorage.getItem('zrsj_persona') as Persona) ?? 'gentle');
  const [apiKey, setApiKey] = useState(localStorage.getItem('zrsj_ai_key') ?? '');
  const [chatOpen, setChatOpen] = useState(false);
  const [vaultSub, setVaultSub] = useState<VaultSub>('vault');
  const [laborSub, setLaborSub] = useState<LaborSub>('temp');
  const [company, setCompany] = useState('');
  const [needCompany, setNeedCompany] = useState(false);
  const [coForm, setCoForm] = useState({
    name: '', tax_no: '', legal_person: '', register_city: '广州',
    tax_city: '广州', bank_name: '', bank_account: '',
  });
  const [coSaving, setCoSaving] = useState(false);
  const [coSeedTasks, setCoSeedTasks] = useState(true);
  const [q, setQ] = useState('');
  const [hits, setHits] = useState<{ kind: string; id: string; title: string; sub: string }[]>([]);
  const [showHits, setShowHits] = useState(false);
  const searchRef = useRef<HTMLDivElement>(null);

  const refresh = async () => {
    try {
      setDash(await api.dashboard());
      setBrief(await api.bizBrief());
      setSeries(await api.statsSeries());
      setTasks(await api.listTasks());
      setDue(await api.dueTasks());
      const c = await api.getCompany();
      setCompany(c?.name ?? '未建档');
      if (c?.name) {
        setNeedCompany(false);
        setCoForm({
          name: c.name ?? '',
          tax_no: c.tax_no ?? '',
          legal_person: c.legal_person ?? '',
          register_city: c.register_city ?? '',
          tax_city: c.tax_city ?? '',
          bank_name: c.bank_name ?? '',
          bank_account: c.bank_account ?? '',
        });
      } else {
        setNeedCompany(true);
      }
      setRefreshKey((k) => k + 1);
    } catch (e) { setToast(String(e)); }
  };
  useEffect(() => { refresh(); }, []);
  useEffect(() => {
    if (!toast) return;
    const t = setTimeout(() => setToast(null), 3200);
    return () => clearTimeout(t);
  }, [toast]);
  useEffect(() => {
    if (!due.length) return;
    try {
      sendNotification({ title: '申报提醒 · 自然拾光记账', body: due.map((d) => `${d.title}（还剩 ${d.days_left} 天）`).join('\n') });
    } catch { /* ignore */ }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [due.length > 0]);

  useEffect(() => {
    if (!q.trim()) { setHits([]); return; }
    const t = setTimeout(async () => {
      try {
        const r = await api.searchAll(q.trim());
        setHits(r);
        setShowHits(true);
      } catch { /* ignore */ }
    }, 180);
    return () => clearTimeout(t);
  }, [q]);

  return (
    <div className="app shell">
      <aside className="side">
        <div className="side-brand">
          <img className="brand-logo" src={logoGold} alt="自然拾光" />
          <div>
            <b>自然拾光记账</b>
            <div className="side-slogan">账清财旺</div>
          </div>
        </div>
        <nav className="side-nav">
          {NAV.map((n) => (
            <div key={n.k} className="nav-group">
              <button
                className={tab === n.k ? 'on' : ''}
                onClick={() => {
                  setTab(n.k);
                  if (n.k === 'vault') setVaultSub(vaultSub);
                }}
              >
                <Icon name={n.icon} />
                <span>{n.label}</span>
              </button>
              {n.children && tab === n.k && (
                <div className="nav-sub">
                  {n.children.map((c) => {
                    const active = n.k === 'vault' ? vaultSub === c.k : laborSub === c.k;
                    return (
                      <button
                        key={c.k}
                        className={active ? 'on' : ''}
                        onClick={() => {
                          setTab(n.k);
                          if (n.k === 'vault') setVaultSub(c.k as VaultSub);
                          else if (n.k === 'jobs') setLaborSub(c.k as LaborSub);
                        }}
                      >
                        <span className="sub-dot" />
                        <span>{c.label}</span>
                      </button>
                    );
                  })}
                </div>
              )}
            </div>
          ))}
        </nav>
        <div className="side-foot">
          <button className={tab === 'settings' ? 'on' : ''} onClick={() => setTab('settings')}>
            <Icon name="gear" />
            <span>设置</span>
          </button>
          <div className="side-note">
            <div className="art-slogan">自然拾光<br />账清财旺</div>
            <div className="art-sub">FORTUNE & CLARITY</div>
          </div>
        </div>
      </aside>

      <div className="main-col">
        <header className="topbar">
          <div className="search-box" ref={searchRef}>
            <Icon name="search" size={16} />
            <input
              placeholder="搜索发票、凭证、流水…"
              value={q}
              onChange={(e) => setQ(e.target.value)}
              onFocus={() => q && setShowHits(true)}
              onBlur={() => setTimeout(() => setShowHits(false), 160)}
            />
            {showHits && hits.length > 0 && (
              <div className="search-hits">
                {hits.map((h, i) => (
                  <div key={i} className="hit">
                    <span className={`tag ${h.kind === 'voucher' ? 'warn' : ''}`}>{h.kind === 'voucher' ? '票据' : '流水'}</span>
                    <div>
                      <b>{h.title}</b>
                      <div className="hint">{h.sub}</div>
                    </div>
                  </div>
                ))}
              </div>
            )}
            {showHits && q && hits.length === 0 && <div className="search-hits"><div className="hit hint">没有找到「{q}」相关内容</div></div>}
          </div>
          <div className="topbar-user">
            <span className="co-name">{company}</span>
            <span className="user-chip">老板</span>
          </div>
        </header>

        <main>
          {tab === 'home' && (
            <Home dash={dash} brief={brief} series={series} tasks={tasks} range={range} setRange={setRange} onGo={(t) => setTab(t)} />
          )}
          {tab === 'work' && <WorkPage onToast={setToast} refreshKey={refreshKey} />}
          {tab === 'accounts' && <AccountPage onToast={setToast} />}
          {tab === 'vault' && <VaultPage onToast={setToast} refreshKey={refreshKey} sub={vaultSub} setSub={setVaultSub} />}
          {tab === 'jobs' && laborSub === 'temp' && <JobsPage onToast={setToast} />}
          {tab === 'jobs' && laborSub === 'emp' && <EmployeePage onToast={setToast} />}
          {tab === 'tax' && <TaxPage onToast={setToast} refreshKey={refreshKey} />}
          {tab === 'report' && <FinReportPage onToast={setToast} />}
        {tab === 'docs' && <DocsPage onToast={setToast} />}
          {tab === 'settings' && <SettingsPage onToast={setToast} />}
        </main>
      </div>

      <Orb
        persona={persona}
        setPersona={(p) => { setPersona(p); localStorage.setItem('zrsj_persona', p); }}
        apiKey={apiKey}
        setApiKey={setApiKey}
        chatOpen={chatOpen}
        onOpenChat={() => setChatOpen((o) => !o)}
      />
      {chatOpen && (
        <AssistantDrawer persona={persona} apiKey={apiKey} onClose={() => setChatOpen(false)} onToast={setToast} />
      )}
      {needCompany && (
        <div className="modal-mask">
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>先建立公司档案</h2>
            <p className="hint">
              记账、开票抬头核对、税费试算都依赖这些信息，先填公司名称和税号即可，其余可稍后补充。
            </p>
            <div className="row">
              <label>公司名称</label>
              <input
                value={coForm.name}
                placeholder="营业执照上的全称"
                onChange={(e) => setCoForm({ ...coForm, name: e.target.value })}
              />
            </div>
            <div className="row">
              <label>纳税人识别号</label>
              <input
                value={coForm.tax_no}
                placeholder="统一社会信用代码（18 位）"
                onChange={(e) => setCoForm({ ...coForm, tax_no: e.target.value })}
              />
            </div>
            <div className="row">
              <label>法定代表人</label>
              <input value={coForm.legal_person} onChange={(e) => setCoForm({ ...coForm, legal_person: e.target.value })} />
            </div>
            <div className="row">
              <label>注册城市</label>
              <input value={coForm.register_city} onChange={(e) => setCoForm({ ...coForm, register_city: e.target.value })} />
            </div>
            <div className="row">
              <label>税务城市</label>
              <input value={coForm.tax_city} onChange={(e) => setCoForm({ ...coForm, tax_city: e.target.value })} />
            </div>
            <div className="row">
              <label>开户行</label>
              <input value={coForm.bank_name} onChange={(e) => setCoForm({ ...coForm, bank_name: e.target.value })} />
            </div>
            <div className="row">
              <label>银行账号</label>
              <input value={coForm.bank_account} onChange={(e) => setCoForm({ ...coForm, bank_account: e.target.value })} />
            </div>
            <div className="row" style={{ marginTop: 10 }}>
              <label>申报任务</label>
              <input type="checkbox" checked={coSeedTasks} onChange={(e) => setCoSeedTasks(e.target.checked)} />
              <span className="hint">按小规模/季度生成常用申报日历（可随时增删）</span>
            </div>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end', marginTop: 14 }}>
              <button className="ghost" onClick={() => setNeedCompany(false)}>稍后再填</button>
              <button
                className="primary"
                disabled={coSaving}
                onClick={async () => {
                  if (!coForm.name.trim()) return setToast('请填写公司名称');
                  if (!coForm.tax_no.trim()) return setToast('请填写纳税人识别号');
                  setCoSaving(true);
                  try {
                    await api.saveCompany(coForm);
                    setCompany(coForm.name);
                    let extra = '';
                    if (coSeedTasks) {
                      try {
                        const n = await api.seedDefaultTasks();
                        if (n > 0) extra = `，并生成 ${n} 条申报任务`;
                      } catch { /* 已生成过则忽略 */ }
                    }
                    setNeedCompany(false);
                    setToast(`公司档案已保存${extra} ✓`);
                    await refresh();
                  } catch (e) {
                    setToast(String(e));
                  } finally {
                    setCoSaving(false);
                  }
                }}
              >
                {coSaving ? '保存中…' : '保存并开始'}
              </button>
            </div>
            <p className="hint" style={{ marginTop: 8 }}>税号用于发票抬头核对，填错会导致「抬头异常」误报。</p>
          </div>
        </div>
      )}
      {toast && <div className="toast">{toast}</div>}
    </div>
  );
}

function Home({
  dash, brief, series, tasks, range, setRange, onGo,
}: {
  dash: Dashboard | null;
  brief: BizBrief | null;
  series: Awaited<ReturnType<typeof api.statsSeries>> | null;
  tasks: TaskRow[];
  range: 'month' | 'year';
  setRange: (r: 'month' | 'year') => void;
  onGo: (t: Tab) => void;
}) {
  const [licExpired, setLicExpired] = useState(false);
  const [licMachine, setLicMachine] = useState('');
  useEffect(() => {
    api.licenseStatus().then((s) => {
      setLicExpired(!s.licensed && s.trial_days_left === 0);
      setLicMachine(s.machine_code);
    }).catch(() => {});
  }, []);
  const today = new Date();
  const daysUntil = (d: string) => Math.ceil((new Date(d).getTime() - today.getTime()) / 86400000);
  const alerts = tasks.filter((t) => t.status !== 'done').map((t) => ({ t, d: daysUntil(t.due_on) })).filter((x) => x.d <= 15);
  const overdue = alerts.filter((x) => x.d < 0);
  const soon = alerts.filter((x) => x.d >= 0);

  // 近 12 个月完整轴（缺月补 0）
  const chartData = (() => {
    const now = new Date();
    const keys: string[] = [];
    for (let i = 11; i >= 0; i--) {
      const d = new Date(now.getFullYear(), now.getMonth() - i, 1);
      keys.push(`${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`);
    }
    const byMonth = new Map((series?.months ?? []).map((m) => [m.month, m]));
    return keys.map((k) => ({
      label: k,
      income: byMonth.get(k)?.income ?? 0,
      expense: byMonth.get(k)?.expense ?? 0,
    }));
  })();
  const [pieMode, setPieMode] = useState<'expense' | 'income'>('expense');
  const pieData = (pieMode === 'expense'
    ? (series?.categories ?? []).map((c) => ({ label: c.category, value: c.expense }))
    : (series?.income_categories ?? []).map((c) => ({ label: c.category, value: c.expense }))
  ).filter((x) => x.value > 0);

  return (
    <>
      {licExpired && (
        <div className="banner overdue">
          ⚠ 试用已结束，功能不受限但建议激活：设置 → 软件授权（机器码 {licMachine}）
        </div>
      )}
      {overdue.length > 0 && (
        <div className="banner overdue">⚠ 有 {overdue.length} 项申报已逾期：{overdue.map((x) => x.t.title).join('、')}——尽快处理，逾期罚款 200–2000 元/次</div>
      )}
      {soon.length > 0 && overdue.length === 0 && (
        <div className="banner due">⏰ 临近申报：{soon.map((x) => `${x.t.title}（还剩 ${x.d} 天）`).join('、')}</div>
      )}
      {!alerts.length && <div className="banner ok">✓ 申报节奏正常，继续保持</div>}

      {/* 快捷入口置顶 */}
      <div className="card">
        <h2>快捷入口</h2>
        <div className="quick">
          <button onClick={() => onGo('work')}>
            <div className="q-title">记一笔</div>
            <div className="q-desc">收支随手记，30 秒完成</div>
            <span className="q-go">→</span>
          </button>
          <button onClick={() => onGo('vault')}>
            <div className="q-title">票夹收票</div>
            <div className="q-desc">邮箱自动收票 / 发票管理</div>
            <span className="q-go">→</span>
          </button>
          <button onClick={() => onGo('jobs')}>
            <div className="q-title">临时用工</div>
            <div className="q-desc">决策树 + 收据一键出</div>
            <span className="q-go">→</span>
          </button>
          <button onClick={() => onGo('tax')}>
            <div className="q-title">申报表格</div>
            <div className="q-desc">正规表单对照填报</div>
            <span className="q-go">→</span>
          </button>
        </div>
      </div>

      {/* 收支趋势卡：KPI 数据整合进卡片头 */}
      <div className="card" style={{ marginTop: 16 }}>
        <div className="trend-head">
          <h2>收支趋势</h2>
          <div className="seg">
            <button className={range === 'month' ? 'on' : ''} onClick={() => setRange('month')}>月度</button>
            <button className={range === 'year' ? 'on' : ''} onClick={() => setRange('year')}>年度</button>
          </div>
        </div>

        <div className="kpi-strip">
          <div>
            <label>本月收入</label>
            <b className="inc">¥{series?.month_income.toFixed(0) ?? dash?.month_income ?? '0'}</b>
          </div>
          <div>
            <label>本月支出</label>
            <b className="exp">¥{series?.month_expense.toFixed(0) ?? dash?.month_expense ?? '0'}</b>
          </div>
          <div>
            <label>账面余额</label>
            <b>¥{brief?.cash_balance.toFixed(0) ?? '0'}</b>
          </div>
          <div>
            <label>90 天现金预测</label>
            <b className={(brief?.forecast_90 ?? 0) < 0 ? 'exp' : 'inc'}>¥{brief?.forecast_90.toFixed(0) ?? '0'}</b>
          </div>
        </div>

        <TrendChart data={chartData} />
        <div className="chart-legend">
          <span><i style={{ background: 'linear-gradient(90deg,#6ee7b7,#2f9e6f)' }} />收入</span>
          <span><i style={{ background: 'linear-gradient(90deg,#f7a8a2,#d9534c)' }} />支出</span>
        </div>
        <div className="grid2" style={{ marginTop: 6 }}>
          <div>
            <p className="hint"><b>柱状对比</b></p>
            <BarChart data={chartData} />
          </div>
          <div>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <p className="hint"><b>收支构成</b></p>
              <div className="seg" style={{ padding: 2 }}>
                <button style={{ padding: '3px 12px', fontSize: 11 }} className={pieMode === 'expense' ? 'on' : ''} onClick={() => setPieMode('expense')}>支出</button>
                <button style={{ padding: '3px 12px', fontSize: 11 }} className={pieMode === 'income' ? 'on' : ''} onClick={() => setPieMode('income')}>收入</button>
              </div>
            </div>
            {pieData.length
              ? <PieChart data={pieData} title={pieMode === 'expense' ? '支出' : '收入'} variant={pieMode} />
              : <p className="hint" style={{ padding: '30px 0' }}>暂无{pieMode === 'expense' ? '支出' : '收入'}数据</p>}
          </div>
        </div>
      </div>

      <div className="grid2 equal" style={{ marginTop: 16 }}>
        <section className="card">
          <h2>经营简报</h2>
          {brief && (
            <>
              <div className="hint">
                <p>💰 现金流预测（90 天）：<b className={brief.forecast_90 < 0 ? 'exp' : 'inc'}>¥{brief.forecast_90.toFixed(2)}</b>
                  {brief.forecast_90 < 0 && <em className="warn"> 触线预警：控制支出</em>}</p>
                <p>📊 临界点：收入 {brief.vat_progress.toFixed(0)} / 30 万，剩余额度 <b>{brief.vat_left.toFixed(0)}</b> 元（只展示事实）</p>
                <p>⏰ {brief.overdue_hint}　｜　Top 支出：{brief.top_expense}</p>
              </div>
              <button className="ghost" onClick={() => onGo('tax')}>去税务申报</button>
            </>
          )}
        </section>
        <section className="card">
          <h2>往来对账（钱到没）</h2>
          <ArList />
          <button className="ghost" onClick={() => onGo('work')}>去工作事项处理</button>
        </section>
      </div>
    </>
  );
}


function ArList() {
  const [rows, setRows] = useState<Awaited<ReturnType<typeof api.arAp>>>([]);
  useEffect(() => { api.arAp().then(setRows); }, []);
  return (
    <ul className="tasks">
      {rows.slice(0, 5).map((r, i) => (
        <li key={i}>
          <div>
            <b>{r.direction === 'receivable' ? '应收' : '应付'} · {r.party}</b>
            <div className="hint">{r.count} 笔 · ¥{r.total.toFixed(2)} · 最早 {r.oldest}</div>
          </div>
          <span className={`tag ${r.direction === 'receivable' ? 'warn' : ''}`}>{r.direction === 'receivable' ? '待收款' : '待付款'}</span>
        </li>
      ))}
      {!rows.length && <li className="ok">往来清爽 ✓</li>}
    </ul>
  );
}
