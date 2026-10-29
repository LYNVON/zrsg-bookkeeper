// 重组后的页面集：工作事项 / 票夹 / 工单 / 税务申报 / 文书 / 设置 / 对话面板
import { useEffect, useRef, useState } from 'react';
import { openPath } from '@tauri-apps/plugin-opener';
import {
  api,
  type AccountRow,
  type CompanyInput,
  type Dashboard,
  type EmployeeRow,
  type EventRow,
  type JobRow,
  type MatchSuggestion,
  type PartyRow,
  type PayrollRow,
  type ReimbursementRow,
  type TaskRow,
  type TaxReport,
  type UnmatchedEvent,
  type BackupItem,
  type FilingDraftRow,
  type FinReport,
  type SiHfConfig,
  type SiHfResult,
  type IntegrityItem,
  type LicenseStatus,
  type MailConfigView,
  type UpdateCheck,
  type VoucherFull,
  type WorklogRow,
} from './api';
import {
  laborWithhold,
  renderCarAgreement,
  renderLaborContract,
  renderLoanNote,
  renderReceipt,
  renderReimbursement,
  renderSettlement,
} from './docs';
import { Mascot } from './orb';

const DIR_LABEL: Record<string, string> = {
  income: '收入', expense: '支出', transfer: '调拨', refund: '冲销',
  loan_in: '借款收', loan_out: '借款出', red_offset: '红冲',
};
const RB_STATUS: Record<string, string> = { none: '未报销', in_reimburse: '报销中', reimbursed: '已报销' };

export async function printHtml(name: string, html: string, onToast: (m: string) => void) {
  const p = await api.exportDocument(name, html);
  await openPath(p);
  onToast('已生成打印页，在浏览器里打印/存 PDF');
}

// ── 工作事项：流水记账完善版（v1.8：筛选/汇总/编辑/删除/多方向） ──

const CAT_CHIPS = ['办公', '油费', '餐费', '交通', '人工费', '房租', '水电', '服务费', '其他'];

export function WorkPage({ onToast, refreshKey }: { onToast: (m: string) => void; refreshKey: number }) {
  const [sub, setSub] = useState<'events' | 'work'>('events');
  const [events, setEvents] = useState<EventRow[]>([]);
  const [unmatched, setUnmatched] = useState<UnmatchedEvent[]>([]);
  const [suggestions, setSuggestions] = useState<MatchSuggestion[]>([]);
  const [accs, setAccs] = useState<{ name: string }[]>([]);
  const [flt, setFlt] = useState({ q: '', direction: '', account: '', from: '', to: '' });
  const [edit, setEdit] = useState<EventRow | null>(null);
  const [showEntry, setShowEntry] = useState(false);
  const [f, setF] = useState({
    direction: 'expense', amount: '', occurred_on: new Date().toISOString().slice(0, 10),
    counterparty: '', memo: '', category: '', account_name: '公户', tax_status: '',
  });

  const reload = async () => {
    setEvents(await api.listEvents(500));
    setUnmatched(await api.listUnmatched());
    setSuggestions(await api.matchSuggestions());
    try {
      const a = await api.listAccounts();
      setAccs(a.map((x) => ({ name: x.name })));
    } catch { /* ignore */ }
  };
  useEffect(() => { reload(); }, [refreshKey]);

  const filtered = events.filter((e) => {
    if (flt.direction && e.direction !== flt.direction) return false;
    if (flt.account && !accs.length) return false;
    if (flt.q) {
      const q = flt.q.toLowerCase();
      const hay = `${e.counterparty ?? ''} ${e.memo ?? ''} ${e.amount} ${e.category ?? ''}`.toLowerCase();
      if (!hay.includes(q)) return false;
    }
    if (flt.from && e.occurred_on < flt.from) return false;
    if (flt.to && e.occurred_on > flt.to) return false;
    return true;
  });
  const sumIn = filtered.filter((e) => e.direction === 'income').reduce((s, e) => s + Number(e.amount), 0);
  const sumOut = filtered.filter((e) => e.direction === 'expense').reduce((s, e) => s + Number(e.amount), 0);

  const submit = async () => {
    if (!f.amount || Number(f.amount) <= 0) return onToast('金额要大于 0');
    await api.addEvent({
      ...f,
      amount: Number(f.amount).toFixed(2),
      counterparty: f.counterparty || null,
      memo: f.memo || null,
      category: f.category || null,
      tax_status: f.tax_status || null,
    });
    onToast('已记一笔 ✓');
    setF({ ...f, amount: '', counterparty: '', memo: '', category: '' });
    await reload();
  };

  return (
    <>
      <div className="subtabs">
        <button className={sub === 'events' ? 'on' : ''} onClick={() => setSub('events')}>流水账（{events.length}）</button>
        <button className={sub === 'work' ? 'on' : ''} onClick={() => setSub('work')}>待处理（{unmatched.length + suggestions.length}）</button>
        <button className="primary" style={{ marginLeft: 'auto' }} onClick={() => setShowEntry(true)}>＋ 记一笔</button>
      </div>

      {sub === 'events' && (
        <div className="card">
          <div className="filter-bar">
            <input placeholder="搜对方/事由/金额/分类…" value={flt.q} onChange={(e) => setFlt({ ...flt, q: e.target.value })} style={{ width: 190 }} />
            <select value={flt.direction} onChange={(e) => setFlt({ ...flt, direction: e.target.value })}>
              <option value="">全部方向</option>
              {Object.entries(DIR_LABEL).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
            </select>
            <input type="date" value={flt.from} onChange={(e) => setFlt({ ...flt, from: e.target.value })} />
            <span className="hint">至</span>
            <input type="date" value={flt.to} onChange={(e) => setFlt({ ...flt, to: e.target.value })} />
            {(flt.q || flt.direction || flt.from || flt.to) && (
              <button className="ghost" onClick={() => setFlt({ q: '', direction: '', account: '', from: '', to: '' })}>清空筛选</button>
            )}
            <span className="hint" style={{ marginLeft: 'auto' }}>
              <b className="inc">收 ¥{sumIn.toFixed(2)}</b> · <b className="exp">支 ¥{sumOut.toFixed(2)}</b> · 净 ¥{(sumIn - sumOut).toFixed(2)} · {filtered.length} 笔
            </span>
          </div>

          <table>
            <thead>
              <tr><th>日期</th><th>方向</th><th>金额</th><th>对方</th><th>事由</th><th>分类</th><th>税务</th><th></th></tr>
            </thead>
            <tbody>
              {filtered.map((e) => (
                <tr key={e.id} className="row-click" onClick={() => setEdit(e)}>
                  <td>{e.occurred_on}</td>
                  <td><span className={`tag ${e.direction === 'income' ? 'income' : e.direction === 'expense' ? 'expense' : ''}`}>{DIR_LABEL[e.direction] ?? e.direction}</span></td>
                  <td className={e.direction === 'income' ? 'inc' : e.direction === 'expense' ? 'exp' : ''}>
                    {e.direction === 'income' ? '+' : e.direction === 'expense' ? '-' : ''}{e.amount}
                  </td>
                  <td>{e.counterparty ?? '—'}</td>
                  <td>{e.memo ?? '—'}</td>
                  <td>{e.category ?? '—'}</td>
                  <td>{e.tax_status ?? <em className="warn">待处理</em>}</td>
                  <td onClick={(ev) => ev.stopPropagation()}>
                    <button className="icon-btn" title="删除" onClick={async () => {
                      if (!confirm('删除这笔流水？')) return;
                      await api.deleteEvent(e.id);
                      onToast('已删除');
                      await reload();
                    }}>✕</button>
                  </td>
                </tr>
              ))}
              {!filtered.length && (
                <tr><td colSpan={8} className="hint" style={{ textAlign: 'center', padding: '40px 0' }}>
                  {events.length ? '没有符合筛选条件的流水' : '还没有记录——点右上角「＋ 记一笔」开始'}
                </td></tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {sub === 'work' && (
        <div className="grid2">
          <section className="card">
            <h2>缺票处置（{unmatched.length}）</h2>
            <ul className="tasks">
              {unmatched.map((e) => {
                const amt = Number(e.amount);
                const d = e.direction === 'income'
                  ? { status: '未开票收入', note: '收入侧：开票或按未开票收入季报' }
                  : amt <= 500
                    ? { status: '收据可扣', note: '≤500 零星 → 收款凭证可税前扣' }
                    : { status: '待补票', note: '先催票；补不上标「不可扣」进汇算调增' };
                return (
                  <li key={e.id}>
                    <div>
                      <b>{e.occurred_on} · ¥{e.amount}</b>
                      <div className="hint">{e.counterparty ?? ''} → <em>{d.note}</em></div>
                    </div>
                    <button className="ghost" onClick={async () => { await api.applyDisposition(e.id, d.status); onToast(`已标记「${d.status}」`); await reload(); }}>{d.status}</button>
                  </li>
                );
              })}
              {!unmatched.length && <li className="ok">干净 ✓</li>}
            </ul>
          </section>
          <section className="card">
            <h2>匹配建议（{suggestions.length}）</h2>
            <ul className="tasks">
              {suggestions.map((s) => (
                <li key={s.voucher_id + s.event_id}>
                  <div>
                    <b>事项 ¥{s.event_amount} ← 票 {s.voucher_no ?? '(无号)'}</b>
                    <div className="hint">{s.reason} · 置信 {s.score}</div>
                  </div>
                  <button className="primary" onClick={async () => { await api.confirmMatch(s.event_id, s.voucher_id); onToast('已挂靠 ✓'); await reload(); }}>挂靠</button>
                </li>
              ))}
              {!suggestions.length && <li className="ok">票据与流水已对齐。</li>}
            </ul>
          </section>
        </div>
      )}

      {/* 记一笔 / 编辑 弹窗 */}
      {(showEntry || edit) && (
        <div className="modal-mask" onClick={() => { setShowEntry(false); setEdit(null); }}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>{edit ? '编辑流水' : '记一笔'}</h2>
            <div className="row">
              <label>方向</label>
              <div className="seg" style={{ flexWrap: 'wrap' }}>
                {Object.entries(DIR_LABEL).map(([k, v]) => (
                  <button key={k} className={(edit ? edit.direction : f.direction) === k ? 'on' : ''}
                    onClick={() => edit ? setEdit({ ...edit, direction: k }) : setF({ ...f, direction: k })}>{v}</button>
                ))}
              </div>
            </div>
            <div className="row"><label>金额</label>
              <input autoFocus inputMode="decimal" value={edit ? edit.amount : f.amount}
                onChange={(e) => edit ? setEdit({ ...edit, amount: e.target.value }) : setF({ ...f, amount: e.target.value })} />
            </div>
            <div className="row"><label>日期</label>
              <input type="date" value={edit ? edit.occurred_on : f.occurred_on}
                onChange={(e) => edit ? setEdit({ ...edit, occurred_on: e.target.value }) : setF({ ...f, occurred_on: e.target.value })} />
            </div>
            <div className="row"><label>对方</label>
              <input value={edit ? edit.counterparty ?? '' : f.counterparty}
                onChange={(e) => edit ? setEdit({ ...edit, counterparty: e.target.value }) : setF({ ...f, counterparty: e.target.value })} />
            </div>
            <div className="row"><label>事由</label>
              <input value={edit ? edit.memo ?? '' : f.memo}
                onChange={(e) => edit ? setEdit({ ...edit, memo: e.target.value }) : setF({ ...f, memo: e.target.value })} />
            </div>
            <div className="row">
              <label>分类</label>
              <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
                {CAT_CHIPS.map((c) => {
                  const cur = edit ? edit.category : f.category;
                  return (
                    <button key={c} className={cur === c ? 'tag income' : 'tag'} style={{ cursor: 'pointer', border: 'none' }}
                      onClick={() => edit ? setEdit({ ...edit, category: c }) : setF({ ...f, category: c })}>{c}</button>
                  );
                })}
                <input style={{ width: 90 }} placeholder="自定义"
                  value={edit ? (edit.category && !CAT_CHIPS.includes(edit.category) ? edit.category : '') : (CAT_CHIPS.includes(f.category) ? '' : f.category)}
                  onChange={(e) => edit ? setEdit({ ...edit, category: e.target.value }) : setF({ ...f, category: e.target.value })} />
              </div>
            </div>
            <div className="row"><label>账户</label>
              <select value={edit ? undefined : f.account_name} onChange={(e) => setF({ ...f, account_name: e.target.value })} disabled={!!edit}>
                {accs.length ? accs.map((a) => <option key={a.name}>{a.name}</option>) : ['公户', '微信', '支付宝', '个人卡', '现金'].map((a) => <option key={a}>{a}</option>)}
              </select>
              {edit && <span className="hint">（编辑时账户不可改——转账/账户语义稳定）</span>}
            </div>
            <div className="row"><label>税务状态</label>
              <select value={edit ? (edit.tax_status ?? '') : f.tax_status}
                onChange={(e) => edit ? setEdit({ ...edit, tax_status: e.target.value }) : setF({ ...f, tax_status: e.target.value })}>
                <option value="">稍后处理</option>
                <option value="可扣">有票可扣</option>
                <option value="收据可扣">收据可扣（小额零星）</option>
                <option value="待补票">待补票</option>
                <option value="不可扣">无票不可扣</option>
                <option value="未开票收入">未开票收入</option>
                <option value="不涉税">不涉税（内部往来）</option>
              </select>
            </div>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
              {edit ? (
                <>
                  <button className="ghost" style={{ color: 'var(--red)' }} onClick={async () => {
                    if (!confirm('删除这笔流水？')) return;
                    await api.deleteEvent(edit.id);
                    onToast('已删除');
                    setEdit(null);
                    await reload();
                  }}>删除</button>
                  <button className="primary" onClick={async () => {
                    await api.updateEvent(edit.id, {
                      occurred_on: edit.occurred_on,
                      direction: edit.direction,
                      amount: Number(edit.amount).toFixed(2),
                      counterparty: edit.counterparty,
                      memo: edit.memo,
                      category: edit.category,
                      tax_status: edit.tax_status,
                    });
                    onToast('已保存 ✓');
                    setEdit(null);
                    await reload();
                  }}>保存修改</button>
                </>
              ) : (
                <>
                  <button className="ghost" onClick={() => setShowEntry(false)}>取消</button>
                  <button className="ghost" onClick={async () => { await submit(); }}>保存后再记</button>
                  <button className="primary" onClick={async () => { await submit(); setShowEntry(false); }}>保存</button>
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}


// ── 票夹（发票 + 报销 + 收票工具） ──────────────────────────

export function VaultPage({
  onToast, refreshKey, sub,
}: {
  onToast: (m: string) => void; refreshKey: number;
  sub: 'vault' | 'rb' | 'intake';
  setSub: (s: 'vault' | 'rb' | 'intake') => void;
}) {
  const [vouchers, setVouchers] = useState<VoucherFull[]>([]);
  const [rbs, setRbs] = useState<ReimbursementRow[]>([]);
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [applicant, setApplicant] = useState('');
  const [docPath, setDocPath] = useState('');
  const [upload, setUpload] = useState(false);
  const [nf, setNf] = useState({ invoice_no: '', invoice_code: '', amount: '', issued_on: new Date().toISOString().slice(0, 10), seller: '', buyer: '' });
  const [importing, setImporting] = useState(false);
  const [badOrig, setBadOrig] = useState<Set<string>>(new Set());
  const [bound, setBound] = useState<MailConfigView | null>(null);
  const [mf, setMf] = useState({ host: 'imap.qq.com', port: 993, user: '', pass: '', folder: 'INBOX', auto: true });
  const autoTried = useRef(false);

  const reload = async () => { setVouchers(await api.listVouchers()); setRbs(await api.listReimbursements()); setSel(new Set()); };
  useEffect(() => { reload(); }, [refreshKey]);
  useEffect(() => {
    api.mailConfigGet().then((c) => {
      setBound(c);
      if (c) setMf({ host: c.host, port: c.port, user: c.user, pass: '', folder: c.folder, auto: c.auto });
    }).catch(() => {});
  }, [refreshKey]);

  // 进入收票页时，若已绑定且开启自动收票，静默拉一次（10 分钟内不重复）
  useEffect(() => {
    if (sub !== 'intake' || !bound?.auto || !bound.has_pass || autoTried.current) return;
    autoTried.current = true;
    api.fetchInvoicesSaved(false)
      .then(async (r) => {
        if (r.imported || r.quarantined) {
          onToast(`自动收票：入夹 ${r.imported} 张${r.quarantined ? ` · 待确认 ${r.quarantined}` : ''}`);
          await reload();
        }
      })
      .catch(() => {});
  }, [sub, bound]);

  const uploadAttachment = async () => {
    if (importing) return;
    try {
      const paths = await api.pickVoucherFiles();
      if (!paths.length) return;
      setImporting(true);
      const rs = await api.importVoucherFiles(paths);
      const ok = rs.filter((r) => r.ok && !r.duplicate).length;
      const dup = rs.filter((r) => r.duplicate).length;
      const bad = rs.filter((r) => !r.ok);
      onToast(
        `已归档 ${ok} 张${dup ? ` · 重复 ${dup}` : ''}${bad.length ? ` · 失败 ${bad.length}（${bad[0].message}）` : ''}`,
      );
      await reload();
    } catch (e) {
      onToast(String(e));
    } finally {
      setImporting(false);
    }
  };

  const allSel = vouchers.length > 0 && vouchers.filter((v) => v.reimburse_status === 'none').every((v) => sel.has(v.id));

  return (
    <>
      {sub === 'vault' && (
        <>
          <div className="toolbar">
            <button className="primary" onClick={uploadAttachment} disabled={importing}>
              {importing ? '导入中…' : '＋ 上传发票附件'}
            </button>
            <button className="ghost" onClick={() => setUpload(true)}>手动录入</button>
            <button
              className="ghost"
              onClick={async () => {
                try {
                  const rs: IntegrityItem[] = await api.verifyVouchers();
                  const bad = rs.filter((r) => !r.ok && r.reason.includes('不一致'));
                  const missing = rs.filter((r) => !r.ok && r.reason.includes('缺失'));
                  setBadOrig(new Set(bad.map((r) => r.id)));
                  onToast(
                    rs.length === 0
                      ? '暂无可校验的原件'
                      : `校验 ${rs.length} 件 · 异常 ${bad.length} · 缺失 ${missing.length}`,
                  );
                } catch (e) { onToast(String(e)); }
              }}
            >校验原件</button>
            <button
              className="ghost"
              onClick={async () => {
                if (!sel.size) return onToast('先勾选发票');
                const id = await api.createReimbursement([...sel], applicant || null, '票夹归集');
                onToast(`报销单已生成（${sel.size} 张票）✓`);
                await api.markReimbursed(id);
                await reload();
              }}
            >
              创建报销单
            </button>
            <button
              className="ghost"
              onClick={() => {
                const selectable = vouchers.filter((v) => v.reimburse_status === 'none');
                setSel(allSel ? new Set() : new Set(selectable.map((v) => v.id)));
              }}
            >
              {allSel ? '取消全选' : '全选'}
            </button>
            <input className="toolbar-input" placeholder="报销人" value={applicant} onChange={(e) => setApplicant(e.target.value)} />
            <span className="hint" style={{ marginLeft: 'auto' }}>共 {vouchers.length} 张 · 已选 {sel.size} 张</span>
          </div>

          {vouchers.length === 0 ? (
            <div className="empty">暂无发票</div>
          ) : (
            <div className="cards-grid">
              {vouchers.map((v) => (
                <div key={v.id} className={sel.has(v.id) ? 'inv-card on' : 'inv-card'}>
                  <label>
                    <input
                      type="checkbox"
                      checked={sel.has(v.id)}
                      disabled={v.reimburse_status !== 'none'}
                      onChange={() => {
                        const s = new Set(sel);
                        s.has(v.id) ? s.delete(v.id) : s.add(v.id);
                        setSel(s);
                      }}
                    />
                    <div style={{ flex: 1 }}>
                      <div className="inv-head">
                        <b>{v.seller ?? '（未知销方）'}</b>
                        <span className="tag warn">{v.expense_category ?? '票据'}</span>
                      </div>
                      <div className="inv-amount">¥{v.amount ?? '—'}</div>
                      <div className="hint">
                        时间：{v.issued_on ?? '—'}<br />
                        票号：{v.invoice_no ?? '(无号)'}
                        <span className="tag" style={{ marginLeft: 8 }}>{RB_STATUS[v.reimburse_status]}</span>
                        {!v.buyer_ok && <em className="warn"> 抬头异常</em>}
                        {badOrig.has(v.id) && <em className="warn"> 原件异常</em>}
                        {v.has_file && v.file_path && (
                          <button
                            className="link-btn"
                            style={{ marginLeft: 8 }}
                            onClick={async (e) => {
                              e.preventDefault();
                              try { await api.openVoucherFile(v.id); } catch (err) { onToast(String(err)); }
                            }}
                          >查看附件</button>
                        )}
                      </div>
                    </div>
                  </label>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {upload && (
        <div className="modal-mask" onClick={() => setUpload(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>手动录入发票</h2>
            <div className="row"><label>发票号码</label><input value={nf.invoice_no} onChange={(e) => setNf({ ...nf, invoice_no: e.target.value })} placeholder="数电票 20 位" /></div>
            <div className="row"><label>发票代码</label><input value={nf.invoice_code} onChange={(e) => setNf({ ...nf, invoice_code: e.target.value })} placeholder="传统票才填" /></div>
            <div className="row"><label>开票日期</label><input type="date" value={nf.issued_on} onChange={(e) => setNf({ ...nf, issued_on: e.target.value })} /></div>
            <div className="row"><label>价税合计</label><input inputMode="decimal" value={nf.amount} onChange={(e) => setNf({ ...nf, amount: e.target.value })} /></div>
            <div className="row"><label>销方</label><input value={nf.seller} onChange={(e) => setNf({ ...nf, seller: e.target.value })} /></div>
            <div className="row"><label>购方</label><input value={nf.buyer} onChange={(e) => setNf({ ...nf, buyer: e.target.value })} placeholder="默认公司档案抬头" /></div>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
              <button className="ghost" onClick={() => setUpload(false)}>取消</button>
              <button
                className="primary"
                onClick={async () => {
                  if (!nf.invoice_no) return onToast('发票号码必填');
                  try {
                    await api.addVoucher({
                      invoice_no: nf.invoice_no,
                      invoice_code: nf.invoice_code || null,
                      amount: nf.amount || null,
                      issued_on: nf.issued_on,
                      seller: nf.seller || null,
                      buyer: nf.buyer || null,
                    });
                    onToast('发票已入夹 ✓');
                    setUpload(false);
                    setNf({ invoice_no: '', invoice_code: '', amount: '', issued_on: new Date().toISOString().slice(0, 10), seller: '', buyer: '' });
                    await reload();
                  } catch (e) { onToast(String(e).includes('UNIQUE') ? '该发票号已存在' : String(e)); }
                }}
              >
                入夹
              </button>
            </div>
          </div>
        </div>
      )}

      {sub === 'rb' && (
        <div>
          <div className="toolbar">
            <b>报销单（{rbs.length}）</b>
            <span className="hint" style={{ marginLeft: 'auto' }}>生成 → 打印 → 标为已报销</span>
          </div>
          <ul className="tasks">
            {rbs.map((r) => (
              <li key={r.id}>
                <div>
                  <b>¥{r.total} · {r.line_count} 张票</b>
                  <div className="hint">{r.form_date} {r.applicant ?? ''} · <span className="tag">{RB_STATUS[r.status] ?? r.status}</span></div>
                </div>
                <div>
                  {r.status !== 'paid' && (
                    <button className="ghost" onClick={async () => { await api.markReimbursed(r.id); onToast('已标「已报销」✓'); await reload(); }}>标为已报销</button>
                  )}
                  <button className="ghost" onClick={async () => {
                    const lines = await api.reimbursementLines(r.id);
                    const html = renderReimbursement({
                      title: '票夹归集报销', applicant: r.applicant ?? '', formDate: r.form_date,
                      lines: lines.map((l) => ({ expense_on: l.expense_on ?? '', category: l.category ?? '费用', invoice_no: l.invoice_no, amount: Number(l.amount ?? 0) })),
                    });
                    await printHtml(`报销单-${r.id.slice(0, 8)}`, html, onToast);
                  }}>打印</button>
                </div>
              </li>
            ))}
            {!rbs.length && <li className="empty" style={{ padding: '60px 0' }}>暂无报销单</li>}
          </ul>
        </div>
      )}

      {sub === 'intake' && (
        <div className="grid2">
          <section className="card">
            <h2>邮箱自动收票</h2>
            {bound ? (
              <>
                <p className="hint">
                  已绑定 <b>{bound.user}</b>（{bound.host}:{bound.port} / {bound.folder}）
                  {bound.last_fetch ? ` · 上次收票 ${new Date(Number(bound.last_fetch) * 1000).toLocaleString()}` : ''}
                </p>
                <div className="row">
                  <label>自动收票</label>
                  <input
                    type="checkbox"
                    checked={bound.auto}
                    onChange={async (e) => {
                      const auto = e.target.checked;
                      await api.mailConfigSave({
                        host: bound.host, port: bound.port, user: bound.user,
                        folder: bound.folder, auto,
                      });
                      setBound({ ...bound, auto });
                      onToast(auto ? '已开启自动收票' : '已关闭自动收票');
                    }}
                  />
                  <span className="hint">进入收票页时自动拉取（10 分钟内不重复）</span>
                </div>
                <div style={{ display: 'flex', gap: 8, marginTop: 10 }}>
                  <button
                    className="primary"
                    onClick={async () => {
                      try {
                        const r = await api.fetchInvoicesSaved(true);
                        onToast(`收票：入夹 ${r.imported} · 待确认 ${r.quarantined} · 跳过 ${r.skipped}${r.errors.length ? ' · ' + r.errors[0] : ''}`);
                        setBound(await api.mailConfigGet());
                        await reload();
                      } catch (e) { onToast(String(e)); }
                    }}
                  >立即收票</button>
                  <button className="ghost" onClick={() => setBound(null)}>重新绑定</button>
                  <button
                    className="ghost"
                    onClick={async () => {
                      await api.mailConfigClear();
                      setBound(null);
                      onToast('已解除邮箱绑定');
                    }}
                  >解除绑定</button>
                </div>
              </>
            ) : (
              <>
                <p className="hint">QQ 邮箱 → 设置 → 账户 → 开启 IMAP 并生成授权码；绑定后自动抽取 PDF 附件入夹</p>
                <div className="row"><label>服务器</label><input value={mf.host} onChange={(e) => setMf({ ...mf, host: e.target.value })} placeholder="imap.qq.com" /></div>
                <div className="row"><label>端口</label><input inputMode="numeric" value={mf.port} onChange={(e) => setMf({ ...mf, port: Number(e.target.value) || 993 })} /></div>
                <div className="row"><label>邮箱账号</label><input value={mf.user} onChange={(e) => setMf({ ...mf, user: e.target.value })} placeholder="you@qq.com" /></div>
                <div className="row"><label>授权码</label><input type="password" value={mf.pass} onChange={(e) => setMf({ ...mf, pass: e.target.value })} placeholder="IMAP 授权码，非登录密码" /></div>
                <div className="row"><label>邮件目录</label><input value={mf.folder} onChange={(e) => setMf({ ...mf, folder: e.target.value })} placeholder="INBOX" /></div>
                <div className="row">
                  <label>自动收票</label>
                  <input type="checkbox" checked={mf.auto} onChange={(e) => setMf({ ...mf, auto: e.target.checked })} />
                  <span className="hint">进入收票页时自动拉取</span>
                </div>
                <button
                  className="primary"
                  onClick={async () => {
                    if (!mf.user || !mf.pass) return onToast('请填写邮箱账号和授权码');
                    try {
                      await api.mailConfigSave({
                        host: mf.host.trim(), port: mf.port, user: mf.user.trim(),
                        pass: mf.pass, folder: mf.folder || 'INBOX', auto: mf.auto,
                      });
                      const r = await api.fetchInvoicesSaved(true);
                      onToast(`绑定成功，收票：入夹 ${r.imported} · 待确认 ${r.quarantined} · 跳过 ${r.skipped}${r.errors.length ? ' · ' + r.errors[0] : ''}`);
                      setBound(await api.mailConfigGet());
                      await reload();
                    } catch (e) { onToast(String(e)); }
                  }}
                >保存并收票</button>
              </>
            )}
          </section>
          <section className="card">
            <h2>外部报销单识别</h2>
            <p className="hint">其他 APP 生成的报销单 PDF → 按发票号匹配票夹 → 自动生成报销单</p>
            <div className="row">
              <label>PDF 路径</label>
              <input value={docPath} onChange={(e) => setDocPath(e.target.value)} placeholder="点右侧选择文件" />
            </div>
            <div style={{ display: 'flex', gap: 8 }}>
              <button
                className="ghost"
                onClick={async () => {
                  const ps = await api.pickVoucherFiles();
                  if (ps.length) setDocPath(ps[0]);
                }}
              >选择文件</button>
              <button
                className="primary"
                onClick={async () => {
                  if (!docPath) return onToast('请先选择 PDF');
                  try { onToast((await api.parseExternalDoc(docPath, null)).hint); await reload(); }
                  catch (e) { onToast(String(e)); }
                }}
              >识别并匹配</button>
            </div>
            <p className="hint" style={{ marginTop: 14 }}>发票清单导入：node tools/import_invoices.mjs &lt;xlsx&gt;</p>
          </section>
        </div>
      )}
    </>
  );
}

// ── 工单（临时用工向导 + 工单 + 协作人） ─────────────────────

export function JobsPage({ onToast }: { onToast: (m: string) => void }) {
  const [worklogs, setWorklogs] = useState<WorklogRow[]>([]);
  const today = new Date().toISOString().slice(0, 10);
  const [company, setCompany] = useState('自然拾光科技（广州）有限公司');
  const [taxNo, setTaxNo] = useState('91440113MAK3PE2J9G');
  const [parties, setParties] = useState<PartyRow[]>([]);
  const [jobs, setJobs] = useState<JobRow[]>([]);
  const [labor, setLabor] = useState({ name: '', idCard: '', reason: '', amount: '', days: '' });
  const [pay, setPay] = useState<{ id: string; amount: string }>({ id: '', amount: '' });

  const reload = async () => { setParties(await api.listParties()); setJobs(await api.listJobs()); };
  useEffect(() => {
    reload();
    api.getCompany().then((c) => { if (c) { setCompany(c.name); setTaxNo(c.tax_no); } });
  }, []);

  const amt = Number(labor.amount || 0);
  const w = laborWithhold(amt);
  const zeroSmall = amt > 0 && amt <= 500;

  return (
    <>
      <div className="grid2">
        <section className="card">
          <h2>临时用工向导</h2>
          <p className="hint">决策树：≤500 零星收款凭证可扣 / 超 500 需代开发票；个税按 61 号公告三级累进预扣</p>
          <div className="row"><label>姓名</label><input value={labor.name} onChange={(e) => setLabor({ ...labor, name: e.target.value })} /></div>
          <div className="row"><label>身份证号</label><input value={labor.idCard} onChange={(e) => setLabor({ ...labor, idCard: e.target.value })} /></div>
          <div className="row"><label>事由/工时</label><input value={labor.reason} onChange={(e) => setLabor({ ...labor, reason: e.target.value })} /></div>
          <div className="row"><label>金额</label><input inputMode="decimal" value={labor.amount} onChange={(e) => setLabor({ ...labor, amount: e.target.value })} /></div>
          <div className="row"><label>工时天数</label><input placeholder="连干多天按事项总额判定" value={labor.days} onChange={(e) => setLabor({ ...labor, days: e.target.value })} /></div>
          {amt > 0 && (
            <div className="hint">
              <b>判定：</b>{zeroSmall ? <span className="ok">≤500 零星 → 《收款收据》✅ 可税前扣</span> : <span className="warn">&gt;500 → 代开发票否则调增 ❌</span>}
              <br /><b>预扣：</b>应纳税所得额 {w.taxable.toFixed(2)} · {w.rate * 100}% − 速扣 {w.quick} = <b>¥{w.withheld.toFixed(2)}</b>
            </div>
          )}
          <div className="seg" style={{ marginTop: 8 }}>
            <button className="ghost" onClick={() => printHtml(`收款收据-${labor.name}`, renderReceipt({ name: labor.name, idCard: labor.idCard, reason: labor.reason, amount: amt, date: today, company }), onToast)}>收款收据</button>
            <button className="ghost" onClick={() => printHtml(`劳务协议-${labor.name}`, renderLaborContract({ company, taxNo, worker: labor.name, idCard: labor.idCard, title: labor.reason, days: labor.days || today, total: amt, date: today }), onToast)}>劳务协议</button>
            <button className="ghost" onClick={() => printHtml(`劳务结算单-${labor.name}`, renderSettlement({ company, worker: labor.name, title: labor.reason, total: amt, paid: 0, tax: w.withheld, date: today }), onToast)}>结算单</button>
          </div>
        </section>

        <section className="card">
          <h2>劳务工单（{jobs.length}）</h2>
          <ul className="tasks">
            {jobs.map((x) => (
              <li key={x.id}>
                <div>
                  <b>{x.title}</b>
                  <div className="hint">{x.worker ?? ''} · 已付 ¥{x.total_paid} · {x.status === 'settled' ? '已结算' : '进行中'}
                    {Number(x.total_paid) > 500 && x.status === 'open' && <em className="warn"> 超零星→需代开发票</em>}
                  </div>
                </div>
                <div>
                  <input style={{ width: 70 }} placeholder="付款" value={pay.id === x.id ? pay.amount : ''} onChange={(e) => setPay({ id: x.id, amount: e.target.value })} />
                  <button className="ghost" onClick={async () => { await api.jobPay(x.id, pay.amount || '0'); onToast('已记付款'); setPay({ id: '', amount: '' }); await reload(); }}>付款</button>
                  <button className="ghost" onClick={async () => { await api.jobSettle(x.id); onToast('已结算 ✓'); await reload(); }}>结算</button>
                  <button
                    className="ghost"
                    onClick={async () => {
                      const today = new Date().toISOString().slice(0, 10);
                      const hours = window.prompt('本次工时（小时，可留空只记出勤）', '8');
                      if (hours === null) return;
                      await api.addWorklog(x.id, today, hours || null, null);
                      onToast('工时已记录 ✓');
                      setWorklogs(await api.listWorklogs(x.id));
                    }}
                  >记工时</button>
                  {worklogs.filter((w) => w.job_id === x.id).length > 0 && (
                    <span className="hint" style={{ marginLeft: 6 }}>
                      已记 {worklogs.filter((w) => w.job_id === x.id).length} 次
                    </span>
                  )}
                </div>
              </li>
            ))}
            {!jobs.length && <li className="hint">暂无工单。</li>}
          </ul>
          <div className="row" style={{ marginTop: 12 }}>
            <label>新开工单</label>
            <input placeholder="活名" id="job-title" />
            <button className="ghost" onClick={async () => {
              const el = document.getElementById('job-title') as HTMLInputElement;
              if (!el?.value) return;
              await api.addJob({ title: el.value, worker_id: null, total_agreed: null });
              el.value = '';
              onToast('工单已创建 ✓');
              await reload();
            }}>创建</button>
          </div>
        </section>
      </div>

      <div className="card" style={{ marginTop: 16 }}>
        <h2>常用协作人（{parties.length}）</h2>
        <p className="hint">熟人反复用工 = 正常经营，建档后免打扰</p>
        <div className="row">
          <label>姓名</label><input id="p-name" />
          <label>身份证号</label><input id="p-id" />
          <button className="ghost" onClick={async () => {
            const n = document.getElementById('p-name') as HTMLInputElement;
            const i = document.getElementById('p-id') as HTMLInputElement;
            if (!n?.value) return;
            await api.addParty({ name: n.value, ptype: 'person', id_card: i?.value || null });
            n.value = ''; if (i) i.value = '';
            onToast('协作人已建档 ✓');
            await reload();
          }}>建档</button>
        </div>
        <ul className="tasks">
          {parties.map((x) => (
            <li key={x.id}><div><b>{x.name}</b><div className="hint">{x.id_card ?? ''} · 累计支出 ¥{x.total_paid ?? '0'}</div></div></li>
          ))}
        </ul>
      </div>
    </>
  );
}

// ── 税务申报（税费计算 + 正规表格 + 日历警告） ──────────────

export function TaxPage({ onToast, refreshKey }: { onToast: (m: string) => void; refreshKey: number }) {
  useEffect(() => { api.listFilingDrafts(20).then(setDrafts).catch(() => {}); }, [refreshKey]);
  const [drafts, setDrafts] = useState<FilingDraftRow[]>([]);
  const [sub, setSub] = useState<'calc' | 'forms' | 'cal'>('cal');
  const [start, setStart] = useState('2026-07-01');
  const [end, setEnd] = useState('2026-09-30');
  const [r, setR] = useState<TaxReport | null>(null);
  const [tasks, setTasks] = useState<TaskRow[]>([]);

  const reload = async () => {
    setR(await api.taxReport(start, end));
    setTasks(await api.listTasks());
  };
  useEffect(() => { reload(); }, [refreshKey]);

  const today = new Date();
  const daysUntil = (d: string) => Math.ceil((new Date(d).getTime() - today.getTime()) / 86400000);

  return (
    <>
      <div className="subtabs">
        <button className={sub === 'cal' ? 'on' : ''} onClick={() => setSub('cal')}>申报日历</button>
        <button className={sub === 'calc' ? 'on' : ''} onClick={() => setSub('calc')}>税费计算</button>
        <button className={sub === 'forms' ? 'on' : ''} onClick={() => setSub('forms')}>申报表格</button>
      </div>

      {sub === 'cal' && (
        <div className="card">
          {tasks.map((t) => {
            const d = daysUntil(t.due_on);
            const cls = t.status === 'done' ? 'ok' : d < 0 ? 'overdue' : d <= 15 ? 'due' : '';
            return (
              <div key={t.id} className={cls && t.status !== 'done' ? `banner ${cls}` : ''} style={{ marginBottom: 8, padding: cls ? undefined : '6px 2px' }}>
                <input type="checkbox" checked={t.status === 'done'} onChange={async () => { await api.toggleTask(t.id); await reload(); }} />
                <div style={{ flex: 1 }}>
                  <b>{t.title}</b>
                  <div className="hint">{t.due_on} · {t.channel ?? ''}</div>
                </div>
                {t.status !== 'done' && (
                  <span className={d < 0 ? 'tag warn' : d <= 15 ? 'tag warn' : 'tag'}>
                    {d < 0 ? `已逾期 ${-d} 天` : d === 0 ? '今天截止' : `还剩 ${d} 天`}
                  </span>
                )}
              </div>
            );
          })}
          {!tasks.length && <p className="hint">日历为空——启动时会自动播种标准申报任务。</p>}
        </div>
      )}

      {sub === 'calc' && (
        <div className="card">
          <h2>税费计算</h2>
          <div className="row">
            <label>所属期</label>
            <input type="date" value={start} onChange={(e) => setStart(e.target.value)} />
            <span>—</span>
            <input type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
            <button className="ghost" onClick={reload}>计算</button>
          </div>
          {r && (
            <table>
              <tbody>
                <tr><td>开票收入（专票口径，税额不免）</td><td className="inc">{r.income_special.toFixed(2)}</td></tr>
                <tr><td>未开票收入（与普票合并免征）</td><td>{r.income_unbilled.toFixed(2)}</td></tr>
                <tr><td>收入合计 / 可扣支出 / 利润</td><td>{r.income_total.toFixed(2)} / {r.expense_deductible.toFixed(2)} / <b>{r.profit.toFixed(2)}</b></td></tr>
                <tr><td>应缴增值税</td><td className="exp">{r.vat_payable.toFixed(2)} <span className="hint">（{r.vat_detail}）</span></td></tr>
                <tr><td>企税预缴（参考）</td><td className="exp">{r.cit_payable.toFixed(2)}</td></tr>
                <tr><td>劳务报酬已预扣</td><td>{r.labor_withheld.toFixed(2)}（{r.labor_count} 笔）</td></tr>
              </tbody>
            </table>
          )}
          {r && <p className={r.expense_nodeduct > 0 ? 'warn' : 'ok'}>{r.note}</p>}
        </div>
      )}

      {sub === 'forms' && (
        <div className="card">
          <h2>申报表格包（对照填报 / 下载留存）</h2>
          <p className="hint">按你的账目自动生成正规申报表数据对照：增值税及附加申报表、企税 A 类预缴表、个税扣缴数据——打印后对照电子税务局填写，或存 PDF 留档。</p>
          <div className="row">
            <label>所属期</label>
            <input type="date" value={start} onChange={(e) => setStart(e.target.value)} />
            <span>—</span>
            <input type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
            <button className="primary" onClick={async () => {
              const p = await api.exportFilingForms(start, end);
              await openPath(p);
              onToast('申报表格包已生成（浏览器打印/存 PDF）');
            }}>生成申报表格包</button>
          </div>
          <button className="ghost" onClick={async () => {
            const p = await api.exportTaxDraft(start, end);
            await openPath(p);
            onToast('季报底稿已生成');
            setDrafts(await api.listFilingDrafts(20));
          }}>生成季报底稿（简版）</button>
          <div style={{ marginTop: 14 }}>
            <div className="hint">底稿历史（{drafts.length}）</div>
            <ul className="tasks">
              {drafts.map((d) => (
                <li key={d.id}>
                  <div>
                    <b>{d.period}</b>
                    <div className="hint">{d.tax_type} · {new Date(Number(d.created_on) * 1000).toLocaleString()}</div>
                  </div>
                </li>
              ))}
              {!drafts.length && <li className="hint">暂无底稿记录</li>}
            </ul>
          </div>
        </div>
      )}
    </>
  );
}

// ── 经营报表（简版：收付实现制） ────────────────────────────

export function FinReportPage({ onToast }: { onToast: (m: string) => void }) {
  const now = new Date();
  const first = new Date(now.getFullYear(), now.getMonth() - 5, 1).toISOString().slice(0, 10);
  const [start, setStart] = useState(first);
  const [end, setEnd] = useState(now.toISOString().slice(0, 10));
  const [rep, setRep] = useState<FinReport | null>(null);
  const [err, setErr] = useState('');

  const load = async () => {
    try {
      setErr('');
      setRep(await api.finReport(start, end));
    } catch (e) { setErr(String(e)); }
  };
  useEffect(() => { load(); /* eslint-disable-next-line */ }, []);

  const money = (n: number) => `¥${n.toFixed(2)}`;

  return (
    <>
      <div className="toolbar">
        <input type="date" value={start} onChange={(e) => setStart(e.target.value)} />
        <span>—</span>
        <input type="date" value={end} onChange={(e) => setEnd(e.target.value)} />
        <button className="primary" onClick={load}>生成报表</button>
        <span className="hint" style={{ marginLeft: 'auto' }}>收付实现制，用于经营判断</span>
      </div>

      {err && <div className="banner overdue">{err}</div>}

      {rep && (
        <>
          <div className="grid4">
            <div className="card"><h2>收入</h2><div className="big">{money(rep.total_income)}</div></div>
            <div className="card"><h2>支出</h2><div className="big">{money(rep.total_expense)}</div></div>
            <div className="card"><h2>经营结余</h2><div className="big">{money(rep.total_profit)}</div></div>
            <div className="card"><h2>应收 / 应付</h2><div className="big">{money(rep.ar)} / {money(rep.ap)}</div></div>
          </div>

          <div className="card" style={{ marginTop: 16 }}>
            <h2>月度收支</h2>
            <table className="table">
              <thead><tr><th>月份</th><th>收入</th><th>支出</th><th>结余</th></tr></thead>
              <tbody>
                {rep.months.map((m) => (
                  <tr key={m.month}>
                    <td>{m.month}</td>
                    <td>{money(m.income)}</td>
                    <td>{money(m.expense)}</td>
                    <td className={m.profit >= 0 ? 'inc' : 'exp'}>{money(m.profit)}</td>
                  </tr>
                ))}
                {!rep.months.length && <tr><td colSpan={4} className="hint">该区间没有收支记录</td></tr>}
              </tbody>
            </table>
          </div>

          <div className="grid2" style={{ marginTop: 16 }}>
            <div className="card">
              <h2>收入构成</h2>
              <ul className="tasks">
                {rep.income_by_cat.map((c) => (
                  <li key={c.category}><span>{c.category}</span><b>{money(c.amount)}</b></li>
                ))}
                {!rep.income_by_cat.length && <li className="hint">无</li>}
              </ul>
            </div>
            <div className="card">
              <h2>支出构成</h2>
              <ul className="tasks">
                {rep.expense_by_cat.map((c) => (
                  <li key={c.category}><span>{c.category}</span><b>{money(c.amount)}</b></li>
                ))}
                {!rep.expense_by_cat.length && <li className="hint">无</li>}
              </ul>
            </div>
          </div>

          <div className="card" style={{ marginTop: 16 }}>
            <h2>账户余额</h2>
            <ul className="tasks">
              {rep.accounts.map((a) => (
                <li key={a.name}><span>{a.name}</span><b>{money(a.balance)}</b></li>
              ))}
              {!rep.accounts.length && <li className="hint">暂无账户</li>}
            </ul>
            <p className="hint">{rep.note}</p>
            <button className="ghost" onClick={() => { onToast('可截图或复制到报表中'); }}>说明</button>
          </div>
        </>
      )}
    </>
  );
}

// ── 文书（私车公用 / 借款借据） ─────────────────────────────

export function DocsPage({ onToast }: { onToast: (m: string) => void }) {
  const today = new Date().toISOString().slice(0, 10);
  const [company, setCompany] = useState('自然拾光科技（广州）有限公司');
  const [taxNo, setTaxNo] = useState('91440113MAK3PE2J9G');
  const [car, setCar] = useState({ owner: '', plate: '', idCard: '' });
  const [loan, setLoan] = useState({ borrower: '', idCard: '', amount: '', dueDate: '2026-12-31', purpose: '' });
  useEffect(() => {
    api.getCompany().then((c) => { if (c) { setCompany(c.name); setTaxNo(c.tax_no); } });
  }, []);

  return (
    <div className="grid2">
      <section className="card">
        <h2>私车公用协议</h2>
        <div className="row"><label>车主</label><input value={car.owner} onChange={(e) => setCar({ ...car, owner: e.target.value })} /></div>
        <div className="row"><label>车牌</label><input value={car.plate} onChange={(e) => setCar({ ...car, plate: e.target.value })} /></div>
        <div className="row"><label>身份证号</label><input value={car.idCard} onChange={(e) => setCar({ ...car, idCard: e.target.value })} /></div>
        <button className="primary" onClick={() => printHtml('私车公用协议', renderCarAgreement({ company, taxNo, owner: car.owner, plate: car.plate, idCard: car.idCard, date: today }), onToast)}>生成协议</button>
      </section>
      <section className="card">
        <h2>借款借据</h2>
        <div className="row"><label>借款人</label><input value={loan.borrower} onChange={(e) => setLoan({ ...loan, borrower: e.target.value })} /></div>
        <div className="row"><label>身份证号</label><input value={loan.idCard} onChange={(e) => setLoan({ ...loan, idCard: e.target.value })} /></div>
        <div className="row"><label>金额</label><input inputMode="decimal" value={loan.amount} onChange={(e) => setLoan({ ...loan, amount: e.target.value })} /></div>
        <div className="row"><label>归还日期</label><input type="date" value={loan.dueDate} onChange={(e) => setLoan({ ...loan, dueDate: e.target.value })} /></div>
        <div className="row"><label>用途</label><input value={loan.purpose} onChange={(e) => setLoan({ ...loan, purpose: e.target.value })} /></div>
        <button className="primary" onClick={() => printHtml('借款借据', renderLoanNote({ company, borrower: loan.borrower, idCard: loan.idCard, amount: Number(loan.amount || 0), dueDate: loan.dueDate, purpose: loan.purpose, date: today }), onToast)}>生成借据</button>
      </section>
    </div>
  );
}

// ── 设置（公司档案 / 数据） ─────────────────────────────────

function SiHfConfigForm({ onToast }: { onToast: (m: string) => void }) {
  const [cfg, setCfg] = useState<SiHfConfig | null>(null);
  const [gross, setGross] = useState('10000');
  const [calc, setCalc] = useState<SiHfResult | null>(null);
  useEffect(() => { api.siHfGet().then(setCfg).catch(() => {}); }, []);
  if (!cfg) return <p className="hint">加载中…</p>;
  const pct = (v: number) => (v * 100).toFixed(2).replace(/\.00$/, '');
  return (
    <>
      <div className="row">
        <label>基数下限</label>
        <input value={cfg.base_min} onChange={(e) => setCfg({ ...cfg, base_min: Number(e.target.value) || 0 })} />
        <label>基数上限</label>
        <input value={cfg.base_max} onChange={(e) => setCfg({ ...cfg, base_max: Number(e.target.value) || 0 })} />
      </div>
      <div className="row">
        <label>养老 %</label>
        <input value={pct(cfg.pension)} onChange={(e) => setCfg({ ...cfg, pension: Number(e.target.value) / 100 || 0 })} />
        <label>医疗 %</label>
        <input value={pct(cfg.medical)} onChange={(e) => setCfg({ ...cfg, medical: Number(e.target.value) / 100 || 0 })} />
        <label>失业 %</label>
        <input value={pct(cfg.unemploy)} onChange={(e) => setCfg({ ...cfg, unemploy: Number(e.target.value) / 100 || 0 })} />
        <label>公积金 %</label>
        <input value={pct(cfg.housing)} onChange={(e) => setCfg({ ...cfg, housing: Number(e.target.value) / 100 || 0 })} />
      </div>
      <button className="primary" onClick={async () => {
        await api.siHfSave(cfg);
        onToast('社保公积金比例已保存 ✓');
      }}>保存比例</button>
      <div className="row" style={{ marginTop: 10 }}>
        <label>试算月薪</label>
        <input inputMode="decimal" value={gross} onChange={(e) => setGross(e.target.value)} />
        <button className="ghost" onClick={async () => { setCalc(await api.siHfCalc(Number(gross) || 0)); }}>测算</button>
      </div>
      {calc && (
        <p className="hint">
          基数 {calc.base.toFixed(0)} ｜ 个人合计 {calc.total.toFixed(2)}（养老 {calc.pension.toFixed(2)} / 医疗 {calc.medical.toFixed(2)} / 失业 {calc.unemploy.toFixed(2)} / 公积金 {calc.housing.toFixed(2)}）｜ 税前收入 {calc.net.toFixed(2)}
        </p>
      )}
      <p className="hint">比例按所在城市社保局口径填写；仅用于工资单与个税估算，不代缴。</p>
    </>
  );
}

export function SettingsPage({ onToast }: { onToast: (m: string) => void }) {
  const [c, setC] = useState<CompanyInput>({ name: '', tax_no: '', legal_person: '', register_city: '广州', tax_city: '广州', bank_name: '', bank_account: '' });
  const [csvPath, setCsvPath] = useState('');
  const [encPass, setEncPass] = useState('');
  const [backups, setBackups] = useState<BackupItem[]>([]);
  const [restorePwd, setRestorePwd] = useState('');
  const [upd] = useState<UpdateCheck | null>(null);
  const [lastAuto, setLastAuto] = useState('');
  const [licSt, setLicSt] = useState<LicenseStatus | null>(null);
  const [licCode, setLicCode] = useState('');
  const loadAux = async () => {
    const [last, list] = await api.backupInfo();
    setLastAuto(last);
    setBackups(list);
    setLicSt(await api.licenseStatus());
  };
  useEffect(() => { api.getCompany().then((v) => v && setC({ ...c, ...v })); loadAux(); /* eslint-disable-line */ }, []);

  return (
    <div className="grid2">
      <section className="card">
        <h2>公司档案</h2>
        {([['name', '公司名称'], ['tax_no', '税号'], ['legal_person', '法定代表人'], ['register_city', '注册城市'], ['tax_city', '税务城市'], ['bank_name', '开户行'], ['bank_account', '银行账号']] as const).map(([k, label]) => (
          <div className="row" key={k}><label>{label}</label><input value={(c[k] as string) ?? ''} onChange={(e) => setC({ ...c, [k]: e.target.value })} /></div>
        ))}
        <div className="row">
          <label>主体类型</label>
          <select value={c.subject_type ?? 'sole'} onChange={(e) => setC({ ...c, subject_type: e.target.value })}>
            <option value="sole">个体工商户</option>
            <option value="individual">个人独资 / 合伙</option>
            <option value="llc">有限责任公司</option>
          </select>
        </div>
        <div className="row">
          <label>是否有雇员</label>
          <input type="checkbox" checked={!!c.has_employee} onChange={(e) => setC({ ...c, has_employee: e.target.checked })} />
          <span className="hint">影响个税扣缴申报提醒</span>
        </div>
        <button className="primary" onClick={async () => {
          await api.saveCompany(c);
          onToast('公司档案已保存，申报日历已按画像重建 ✓');
        }}>保存档案</button>
        <p className="hint">注册城市与税务城市可不同；保存后会重新生成「该报的」申报提醒。</p>
      </section>
      <section className="card">
        <h2>数据与备份</h2>
        <div className="row"><label>备份</label><button className="ghost" onClick={async () => { const p = await api.backupNow(); onToast(`备份完成：${p}`); await loadAux(); }}>立即备份</button></div>
        <div className="hint">
          自动备份：{lastAuto ? new Date(Number(lastAuto) * 1000).toLocaleString() : '尚未执行（启动时每小时补一次，保留 20 份）'}
        </div>
        <div className="row" style={{ marginTop: 6 }}>
          <label>诊断</label>
          <button className="ghost" onClick={async () => { await api.openLogsDir(); }}>打开日志目录</button>
          <span className="hint">崩溃会写入 logs/crash.log</span>
        </div>
        <div className="row" style={{ marginTop: 8 }}>
          <label>恢复</label>
          <select
            value=""
            onChange={async (e) => {
              const item = backups.find((b) => b.path === e.target.value);
              if (!item) return;
              if (item.kind === 'encrypted') {
                if (!restorePwd) return onToast('加密备份需先填写下方口令');
                if (!confirm(`将用「${item.name}」覆盖当前数据（会先自动备份当前库），继续？`)) return;
                try {
                  const snap = await api.restoreEncrypted(item.path, restorePwd);
                  onToast(`已恢复；恢复前快照：${snap}`);
                  await loadAux();
                } catch (err) { onToast(String(err)); }
              } else {
                if (!confirm(`将用「${item.name}」覆盖当前数据（会先自动备份当前库），继续？`)) return;
                try {
                  const snap = await api.restoreBackup(item.path);
                  onToast(`已恢复；恢复前快照：${snap}`);
                  await loadAux();
                } catch (err) { onToast(String(err)); }
              }
            }}
          >
            <option value="">选择备份文件恢复…</option>
            {backups.map((b) => (
              <option key={b.path} value={b.path}>
                {b.name} · {(b.size / 1024).toFixed(0)}KB · {b.kind === 'encrypted' ? '加密' : '明文'}
              </option>
            ))}
            {!backups.length && <option value="">（暂无备份）</option>}
          </select>
        </div>
        <div className="row"><label>加密备份</label><input type="password" placeholder="AES-256-GCM 口令" value={encPass} onChange={(e) => setEncPass(e.target.value)} /></div>
        <button className="ghost" onClick={async () => { if (!encPass) return onToast('请设置口令'); const p = await api.backupEncrypted(encPass); onToast(`加密备份：${p}`); await loadAux(); }}>生成加密备份包</button>
        <div className="row" style={{ marginTop: 8 }}><label>恢复口令</label><input type="password" placeholder="恢复加密备份时使用" value={restorePwd} onChange={(e) => setRestorePwd(e.target.value)} /></div>
        <div className="row" style={{ marginTop: 12 }}>
          <label>换机迁移</label>
          <button className="ghost" onClick={async () => {
            try {
              const p = await api.exportAllData();
              onToast(`已导出全部数据：${p}`);
            } catch (e) { onToast(String(e)); }
          }}>导出全部数据</button>
          <button className="ghost" onClick={async () => {
            const f = await api.pickDbFile();
            if (!f) return;
            if (!confirm(`将用 ${f} 覆盖当前账套（会先自动备份当前库），继续？`)) return;
            try {
              const snap = await api.restoreBackup(f);
              onToast(`已恢复；恢复前快照：${snap}`);
              await loadAux();
            } catch (e) { onToast(String(e)); }
          }}>从文件恢复</button>
        </div>
        <p className="hint">导出内容：zrsj.db（完整账套）+ attachments（原件）+ manifest.json</p>
        <div className="row" style={{ marginTop: 12 }}><label>CSV 流水</label><input value={csvPath} onChange={(e) => setCsvPath(e.target.value)} placeholder="CSV 完整路径" /></div>
        <button className="ghost" onClick={async () => { if (!csvPath) return onToast('请填路径'); onToast(`已导入 ${await api.importCsv(csvPath)} 笔`); }}>导入流水</button>
        <p className="hint">Excel 迁移：node tools/migrate_workbook.mjs　｜　发票导入：node tools/import_invoices.mjs</p>
        <div className="side-note" style={{ padding: '12px 0 0' }}>
          {upd && <div className="hint">上次检查：{upd.message}</div>}
          <div className="row" style={{ marginTop: 10 }}>
            <button
              className="primary"
              onClick={async () => {
                try {
                  const { check } = await import('@tauri-apps/plugin-updater');
                  const up = await check();
                  if (!up) return onToast('已是最新版本 ✓');
                  const go = confirm(
                    `发现新版本 ${up.currentVersion} → ${up.version}\n${up.body ?? ''}\n\n现在后台下载并安装？（完成后会自动重启）`,
                  );
                  if (!go) return;
                  onToast('正在后台下载更新…');
                  await up.downloadAndInstall();
                  onToast('安装完成，正在重启…');
                  const { relaunch } = await import('@tauri-apps/plugin-process');
                  await relaunch();
                } catch (e) {
                  const msg = String(e);
                  onToast(
                    msg.includes('endpoints') || msg.includes('No endpoints')
                      ? '当前构建未配置更新源，无法检查更新'
                      : `检查更新失败：${msg.slice(0, 90)}`,
                  );
                }
              }}
            >检查并更新</button>
            <span className="hint">静默下载安装，完成后自动重启；数据保留</span>
          </div>
        </div>
      </section>
      <section className="card">
        <h2>社保公积金（个人部分比例）</h2>
        <SiHfConfigForm onToast={onToast} />
      </section>
      <section className="card">
        <h2>软件授权</h2>
        <div className="hint">
          机器码：<b>{licSt?.machine_code ?? '—'}</b>
          <button className="link-btn" style={{ marginLeft: 8 }} onClick={async () => {
            if (licSt?.machine_code) {
              await navigator.clipboard.writeText(licSt.machine_code);
              onToast('机器码已复制，发给开发者换取授权码');
            }
          }}>复制</button>
        </div>
        <div className="hint" style={{ marginTop: 6 }}>
          状态：<b>{licSt ? (licSt.licensed ? `已授权（${licSt.plan}${licSt.expire_on ? ` · 到期 ${licSt.expire_on}` : ' · 永久'}）` : licSt.message) : '—'}</b>
        </div>
        <div className="row" style={{ marginTop: 10 }}>
          <label>授权码</label>
          <input value={licCode} placeholder="粘贴授权码" onChange={(e) => setLicCode(e.target.value)} />
          <button className="primary" onClick={async () => {
            if (!licCode.trim()) return onToast('请粘贴授权码');
            try {
              const s = await api.licenseActivate(licCode.trim());
              setLicSt(s);
              onToast(s.message);
            } catch (e) { onToast(String(e)); }
          }}>激活</button>
        </div>
        <p className="hint">授权码绑定本机机器码；换电脑需要重新签发。离线校验，不联网。</p>
      </section>
    </div>
  );
}

// ── 正式员工：工资绩效 + 报销联动（v1.5） ──────────────────

export function EmployeePage({ onToast }: { onToast: (m: string) => void }) {
  const [emps, setEmps] = useState<EmployeeRow[]>([]);
  const [payrolls, setPayrolls] = useState<PayrollRow[]>([]);
  const [f, setF] = useState({ name: '', id_card: '', title: '', hire_on: new Date().toISOString().slice(0, 10), base_salary: '' });
  const [p, setP] = useState({ employee_id: '', period_month: new Date().toISOString().slice(0, 7), gross: '', si_hf: '', tax: '', net: '' });
  const [bonus, setBonus] = useState({ employee_id: '', amount: '', memo: '' });
  const [openRb, setOpenRb] = useState<string | null>(null); // 员工 id：展开报销明细
  const [rbs, setRbs] = useState<ReimbursementRow[]>([]);

  const reload = async () => {
    setEmps(await api.listEmployees());
    setPayrolls(await api.listPayroll());
    setRbs(await api.listReimbursements());
  };
  useEffect(() => { reload(); }, []);

  return (
    <>
      <div className="grid2">
        <section className="card">
          <h2>员工档案（{emps.length}）</h2>
          <p className="hint">正式员工建档 → 工资单/绩效/报销三线联动</p>
          <div className="row"><label>姓名</label><input value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} /></div>
          <div className="row"><label>身份证号</label><input value={f.id_card} onChange={(e) => setF({ ...f, id_card: e.target.value })} /></div>
          <div className="row"><label>岗位</label><input value={f.title} onChange={(e) => setF({ ...f, title: e.target.value })} placeholder="如：运营" /></div>
          <div className="row"><label>入职日期</label><input type="date" value={f.hire_on} onChange={(e) => setF({ ...f, hire_on: e.target.value })} /></div>
          <div className="row"><label>底薪</label><input inputMode="decimal" value={f.base_salary} onChange={(e) => setF({ ...f, base_salary: e.target.value })} /></div>
          <button className="primary" onClick={async () => {
            if (!f.name) return onToast('姓名必填');
            await api.addEmployee({ ...f, id_card: f.id_card || null, title: f.title || null, base_salary: f.base_salary || null });
            onToast('员工已建档 ✓');
            setF({ name: '', id_card: '', title: '', hire_on: new Date().toISOString().slice(0, 10), base_salary: '' });
            await reload();
          }}>建档</button>
        </section>

        <section className="card">
          <h2>工资单录入</h2>
          <p className="hint">累计预扣法自动算税（7 级税率），同人同期只录一次</p>
          <div className="row">
            <label>员工</label>
            <select value={p.employee_id} onChange={(e) => setP({ ...p, employee_id: e.target.value })}>
              <option value="">—</option>
              {emps.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
            </select>
          </div>
          <div className="row"><label>属期</label><input value={p.period_month} onChange={(e) => setP({ ...p, period_month: e.target.value })} placeholder="2026-10" /></div>
          <div className="row"><label>应发工资</label><input inputMode="decimal" value={p.gross} onChange={(e) => setP({ ...p, gross: e.target.value })} /></div>
          <div className="row"><label>社保公积金</label><input inputMode="decimal" value={p.si_hf} onChange={(e) => setP({ ...p, si_hf: e.target.value })} /></div>
          <div className="row">
            <label>个税</label>
            <input inputMode="decimal" value={p.tax} onChange={(e) => setP({ ...p, tax: e.target.value })} />
            <button className="ghost" onClick={async () => {
              if (!p.employee_id || !p.gross) return onToast('先选员工填应发');
              const t = await api.wageTaxPreview(p.employee_id, p.period_month, p.gross, p.si_hf || '0');
              const net = (Number(p.gross) - Number(p.si_hf || 0) - t).toFixed(2);
              setP({ ...p, tax: t.toFixed(2), net });
              onToast(`累计预扣：个税 ¥${t.toFixed(2)} · 实发 ¥${net}`);
            }}>自动算税</button>
          </div>
          <div className="row"><label>实发</label><input inputMode="decimal" value={p.net} onChange={(e) => setP({ ...p, net: e.target.value })} /></div>
          <button className="primary" onClick={async () => {
            if (!p.employee_id || !p.gross) return onToast('员工与应发必填');
            try {
              await api.addPayroll(p.employee_id, p.period_month, p.gross, p.si_hf || '0', p.tax || '0', p.net || p.gross);
              onToast('工资单已录入，实发已生成支出事项 ✓');
              setP({ ...p, gross: '', si_hf: '', tax: '', net: '' });
              await reload();
            } catch (e) { onToast(String(e)); }
          }}>录入工资单</button>
        </section>
      </div>

      <div className="grid2" style={{ marginTop: 16 }}>
        <section className="card">
          <h2>绩效 / 奖金</h2>
          <div className="row">
            <label>员工</label>
            <select value={bonus.employee_id} onChange={(e) => setBonus({ ...bonus, employee_id: e.target.value })}>
              <option value="">—</option>
              {emps.map((x) => <option key={x.id} value={x.id}>{x.name}</option>)}
            </select>
          </div>
          <div className="row"><label>金额</label><input inputMode="decimal" value={bonus.amount} onChange={(e) => setBonus({ ...bonus, amount: e.target.value })} /></div>
          <div className="row"><label>说明</label><input value={bonus.memo} onChange={(e) => setBonus({ ...bonus, memo: e.target.value })} placeholder="如：Q3 绩效" /></div>
          <button className="primary" onClick={async () => {
            if (!bonus.employee_id || !bonus.amount) return onToast('员工与金额必填');
            await api.addBonus(bonus.employee_id, bonus.amount, bonus.memo || '绩效奖金');
            onToast('绩效/奖金已生成支出事项 ✓');
            setBonus({ employee_id: '', amount: '', memo: '' });
          }}>录入绩效</button>
          <p className="hint" style={{ marginTop: 12 }}>绩效与工资分录独立，均计入「工资薪金」类支出，个税并入工资累计预扣（发工资时申报）。</p>
        </section>

        <section className="card">
          <h2>工资单记录（{payrolls.length}）</h2>
          <ul className="tasks">
            {payrolls.map((x) => (
              <li key={x.id}>
                <div>
                  <b>{x.employee} · {x.period_month}</b>
                  <div className="hint">应发 ¥{x.gross} · 社保 ¥{x.si_hf} · 个税 ¥{x.tax} · 实发 ¥{x.net}</div>
                </div>
              </li>
            ))}
            {!payrolls.length && <li className="hint">暂无工资单。</li>}
          </ul>
        </section>
      </div>

      {emps.length > 0 && (
        <div className="card" style={{ marginTop: 16 }}>
          <h2>员工卡片 · 报销联动</h2>
          <div className="cards-grid">
            {emps.map((x) => (
              <div key={x.id} className="inv-card">
                <div className="inv-head">
                  <b>{x.name}</b>
                  <span className="tag">{x.title ?? '员工'}</span>
                </div>
                <div className="hint">
                  身份证：{x.id_card ?? '—'}<br />
                  入职：{x.hire_on ?? '—'} · 底薪 ¥{x.base_salary ?? '—'}
                </div>
                <div className="inv-amount" style={{ fontSize: 15 }}>
                  报销 {x.rb_count} 笔 · 合计 ¥{x.rb_total}
                  {Number(x.advance_open) > 0 && <em className="warn"> 未清垫付 ¥{x.advance_open}</em>}
                </div>
                <button className="ghost" onClick={() => setOpenRb(openRb === x.id ? null : x.id)}>
                  {openRb === x.id ? '收起' : '查看TA的报销单'}
                </button>
                {openRb === x.id && (
                  <ul className="tasks" style={{ marginTop: 8 }}>
                    {rbs.filter((r) => r.applicant === x.name).map((r) => (
                      <li key={r.id}>
                        <div>
                          <b>¥{r.total}</b>
                          <div className="hint">{r.form_date} · {r.line_count} 张票 · {r.status}</div>
                        </div>
                      </li>
                    ))}
                    {!rbs.filter((r) => r.applicant === x.name).length && <li className="hint">暂无报销记录</li>}
                  </ul>
                )}
              </div>
            ))}
          </div>
          <p className="hint" style={{ marginTop: 10 }}>
            联动规则：在「票夹 → 报销单」创建时填入员工姓名，即自动挂到该员工卡片下；未清垫付 = 该员工名下未报销的费用事项。
          </p>
        </div>
      )}
    </>
  );
}

// ── 账户模块：总览 / 详情（快捷入口+设置六件套） / 转账（v1.7） ──

const ACCT_TYPE: Record<string, string> = {
  bank: '银行', wechat: '微信', alipay: '支付宝', personal_card: '个人卡', cash: '现金',
};

export function AccountPage({ onToast }: { onToast: (m: string) => void }) {
  const [accs, setAccs] = useState<AccountRow[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [evs, setEvs] = useState<EventRow[]>([]);
  const [menuOpen, setMenuOpen] = useState(false);
  const [modal, setModal] = useState<'' | 'add' | 'edit' | 'entry' | 'recon' | 'assets' | 'transfers' | 'del'>('');
  const [nf, setNf] = useState({ name: '', atype: 'bank', opening_balance: '0', opening_date: new Date().toISOString().slice(0, 10), is_company: true });
  const [tr, setTr] = useState({ from_id: '', to_id: '', amount: '', occurred_on: new Date().toISOString().slice(0, 10), memo: '' });
  const [entry, setEntry] = useState({ direction: 'expense', amount: '', memo: '', category: '', tax_status: '' });
  const [realBal, setRealBal] = useState('');
  const [tlogs, setTlogs] = useState<EventRow[]>([]);

  const cur = accs.find((a) => a.id === openId) ?? null;
  const reload = async () => {
    setAccs(await api.listAccounts());
    if (openId) setEvs(await api.accountEvents(openId, 50));
  };
  useEffect(() => { reload(); }, []);
  useEffect(() => {
    if (!openId) return;
    api.accountEvents(openId, 50).then(setEvs);
  }, [openId]);

  const close = () => { setModal(''); setMenuOpen(false); };

  return (
    <>
      <div className="toolbar">
        <button className="primary" onClick={() => { setNf({ name: '', atype: 'bank', opening_balance: '0', opening_date: new Date().toISOString().slice(0, 10), is_company: true }); setModal('add'); }}>＋ 新增账户</button>
        <span className="hint" style={{ marginLeft: 'auto' }}>点账户卡进入详情 · 快捷记账与账户设置</span>
      </div>

      <div className="cards-grid">
        {accs.map((a) => (
          <div key={a.id} className={openId === a.id ? 'inv-card on' : 'inv-card'}
            onContextMenu={(e) => { e.preventDefault(); setOpenId(a.id); setMenuOpen(true); }}>
            <div className="inv-head">
              <b>{a.name}</b>
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <span className={`tag ${a.is_company ? '' : 'warn'}`}>{ACCT_TYPE[a.atype] ?? a.atype} · {a.is_company ? '公司' : '个人'}</span>
                {openId === a.id && (
                  <button className="icon-btn" title="账户设置" onClick={(e) => { e.stopPropagation(); setMenuOpen(!menuOpen); }}>⚙</button>
                )}
              </div>
            </div>
            <div className={a.balance < 0 ? 'inv-amount exp' : 'inv-amount'}>¥{a.balance.toFixed(2)}</div>
            <div className="hint">期初 ¥{a.opening_balance} {a.opening_date ? `· ${a.opening_date}` : ''} · 流水 {a.event_count} 笔</div>
            <button className="ghost" onClick={() => setOpenId(openId === a.id ? null : a.id)}>{openId === a.id ? '收起' : '进入账户'}</button>

            {openId === a.id && cur && (
            <div className="acc-detail">
              <div className="acc-quick">
                <button className="primary" onClick={() => { setEntry({ direction: 'expense', amount: '', memo: '', category: '', tax_status: '' }); setModal('entry'); }}>＋ 记一笔</button>
                <button className="ghost" onClick={() => { setTr({ from_id: a.id, to_id: '', amount: '', occurred_on: new Date().toISOString().slice(0, 10), memo: '' }); document.getElementById('acc-transfer')?.scrollIntoView({ behavior: 'smooth' }); }}>转账</button>
                {menuOpen && (
                  <div className="orb-menu acc-menu">
                    {([
                      ['edit', '编辑账户'],
                      ['export', '流水导出'],
                      ['recon', '流水对账'],
                      ['assets', '资产明细'],
                      ['transfers', '转账记录'],
                      ['del', '删除账户'],
                    ] as const).map(([k, label]) => (
                      <button key={k} className="q-chip" style={{ marginBottom: 4, padding: '9px 12px', justifyContent: 'flex-start' }}
                        onClick={() => {
                          setMenuOpen(false);
                          if (k === 'export') {
                            const csv = '日期,方向,金额,对方,事由,分类\n' + evs.map((e) => `${e.occurred_on},${e.direction},${e.amount},${e.counterparty ?? ''},${e.memo ?? ''},${e.category ?? ''}`).join('\n');
                            api.exportText(`流水-${a.name}`, csv, 'csv').then((p) => { onToast(`已导出：${p}`); openPath(p); });
                          } else if (k === 'transfers') {
                            api.accountTransfers(a.id).then(setTlogs);
                            setModal('transfers');
                          } else {
                            setModal(k);
                            if (k === 'edit') setNf({ name: a.name, atype: a.atype, opening_balance: a.opening_balance, opening_date: a.opening_date ?? '', is_company: a.is_company });
                          }
                        }}>
                        {label}
                      </button>
                    ))}
                  </div>
                )}
              </div>
              <ul className="tasks" style={{ marginTop: 10 }}>
                {evs.map((e) => (
                  <li key={e.id}>
                    <div>
                      <b>{e.occurred_on} · {DIR_LABEL[e.direction] ?? e.direction}</b>
                      <div className="hint">{e.counterparty ?? ''} {e.memo ?? ''}</div>
                    </div>
                    <span className={e.direction === 'income' ? 'inc' : e.direction === 'expense' ? 'exp' : ''}>
                      {e.direction === 'income' ? '+' : e.direction === 'expense' ? '-' : ''}{e.amount}
                    </span>
                  </li>
                ))}
                {!evs.length && <li className="hint">该账户暂无流水</li>}
              </ul>
            </div>
          )}
          </div>
        ))}
        {!accs.length && <div className="empty">暂无账户</div>}
      </div>

      <div className="card" id="acc-transfer" style={{ marginTop: 16 }}>
        <h2>账户转账</h2>
        <p className="hint">生成配对调拨事项（不计入收支与税），余额一增一减</p>
        <div className="row">
          <label>转出</label>
          <select value={tr.from_id} onChange={(e) => setTr({ ...tr, from_id: e.target.value })}>
            <option value="">—</option>
            {accs.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
          </select>
          <label>转入</label>
          <select value={tr.to_id} onChange={(e) => setTr({ ...tr, to_id: e.target.value })}>
            <option value="">—</option>
            {accs.map((a) => <option key={a.id} value={a.id}>{a.name}</option>)}
          </select>
        </div>
        <div className="row">
          <label>金额</label>
          <input inputMode="decimal" value={tr.amount} onChange={(e) => setTr({ ...tr, amount: e.target.value })} />
          <label>日期</label>
          <input type="date" value={tr.occurred_on} onChange={(e) => setTr({ ...tr, occurred_on: e.target.value })} />
          <label>备注</label>
          <input value={tr.memo} onChange={(e) => setTr({ ...tr, memo: e.target.value })} />
        </div>
        <button className="primary" onClick={async () => {
          if (!tr.from_id || !tr.to_id || !tr.amount) return onToast('转出/转入/金额必填');
          try {
            await api.transfer(tr.from_id, tr.to_id, Number(tr.amount).toFixed(2), tr.occurred_on, tr.memo || null);
            onToast('转账已配对入账 ✓');
            setTr({ ...tr, amount: '', memo: '' });
            await reload();
          } catch (e) { onToast(String(e)); }
        }}>确认转账</button>
      </div>

      {/* ── 弹窗区 ── */}
      {(modal === 'add' || modal === 'edit') && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>{modal === 'add' ? '新增账户' : '编辑账户'}</h2>
            <div className="row"><label>账户名</label><input value={nf.name} onChange={(e) => setNf({ ...nf, name: e.target.value })} /></div>
            <div className="row">
              <label>类型</label>
              <select value={nf.atype} onChange={(e) => setNf({ ...nf, atype: e.target.value })}>
                {Object.entries(ACCT_TYPE).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
              </select>
            </div>
            <div className="row"><label>期初余额</label><input inputMode="decimal" value={nf.opening_balance} onChange={(e) => setNf({ ...nf, opening_balance: e.target.value })} /></div>
            <div className="row"><label>期初日期</label><input type="date" value={nf.opening_date} onChange={(e) => setNf({ ...nf, opening_date: e.target.value })} /></div>
            <div className="row">
              <label>归属</label>
              <select value={nf.is_company ? 'company' : 'personal'} onChange={(e) => setNf({ ...nf, is_company: e.target.value === 'company' })}>
                <option value="company">公司账户</option>
                <option value="personal">个人账户</option>
              </select>
            </div>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
              <button className="ghost" onClick={close}>取消</button>
              <button className="primary" onClick={async () => {
                if (!nf.name) return onToast('账户名必填');
                if (modal === 'add') {
                  await api.addAccount({ ...nf, opening_date: nf.opening_date || null });
                  onToast('账户已创建 ✓');
                } else if (openId) {
                  await api.updateAccount(openId, { ...nf, opening_date: nf.opening_date || null });
                  onToast('账户已更新 ✓');
                }
                close();
                await reload();
              }}>{modal === 'add' ? '创建' : '保存'}</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'entry' && cur && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>记一笔到「{cur.name}」</h2>
            <div className="row">
              <label>方向</label>
              <div className="seg">
                {(['income', 'expense'] as const).map((d) => (
                  <button key={d} className={entry.direction === d ? 'on' : ''} onClick={() => setEntry({ ...entry, direction: d })}>{DIR_LABEL[d]}</button>
                ))}
              </div>
            </div>
            <div className="row"><label>金额</label><input autoFocus inputMode="decimal" value={entry.amount} onChange={(e) => setEntry({ ...entry, amount: e.target.value })} /></div>
            <div className="row"><label>事由</label><input value={entry.memo} onChange={(e) => setEntry({ ...entry, memo: e.target.value })} /></div>
            <div className="row"><label>分类</label><input value={entry.category} onChange={(e) => setEntry({ ...entry, category: e.target.value })} /></div>
            <div className="row">
              <label>税务状态</label>
              <select value={entry.tax_status} onChange={(e) => setEntry({ ...entry, tax_status: e.target.value })}>
                <option value="">稍后处理</option>
                <option value="可扣">有票可扣</option>
                <option value="收据可扣">收据可扣</option>
                <option value="待补票">待补票</option>
                <option value="不可扣">不可扣</option>
                <option value="未开票收入">未开票收入</option>
              </select>
            </div>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
              <button className="ghost" onClick={close}>取消</button>
              <button className="primary" onClick={async () => {
                if (!entry.amount) return onToast('金额必填');
                await api.addEvent({
                  occurred_on: new Date().toISOString().slice(0, 10),
                  direction: entry.direction,
                  amount: Number(entry.amount).toFixed(2),
                  memo: entry.memo || null,
                  category: entry.category || null,
                  tax_status: entry.tax_status || null,
                  account_name: cur.name,
                });
                onToast('已记一笔 ✓');
                close();
                await reload();
              }}>保存</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'recon' && cur && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>流水对账 · {cur.name}</h2>
            <table>
              <tbody>
                <tr><td>期初余额</td><td>¥{cur.opening_balance}</td></tr>
                <tr><td>流水轧差（收−支）</td><td>¥{(cur.balance - Number(cur.opening_balance)).toFixed(2)}</td></tr>
                <tr><td><b>账面余额</b></td><td><b>¥{cur.balance.toFixed(2)}</b></td></tr>
              </tbody>
            </table>
            <div className="row" style={{ marginTop: 12 }}>
              <label>银行实际余额</label>
              <input inputMode="decimal" value={realBal} onChange={(e) => setRealBal(e.target.value)} placeholder="登录网银核对后填写" />
            </div>
            {realBal && (
              <p className={Math.abs(Number(realBal) - cur.balance) < 0.01 ? 'ok' : 'warn'}>
                {Math.abs(Number(realBal) - cur.balance) < 0.01
                  ? '✓ 对平！账面与银行一致（差 1 分也要查平的规矩达成）'
                  : `⚠ 差异 ¥${(Number(realBal) - cur.balance).toFixed(2)} —— 请逐笔核对未达账项（手续费/利息/在途）`}
              </p>
            )}
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button className="primary" onClick={close}>完成</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'assets' && cur && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>资产明细 · {cur.name}</h2>
            <table>
              <tbody>
                <tr><td>期初资产</td><td>¥{cur.opening_balance}</td></tr>
                <tr><td>累计收入</td><td className="inc">+¥{evs.filter((e) => e.direction === 'income').reduce((s, e) => s + Number(e.amount), 0).toFixed(2)}</td></tr>
                <tr><td>累计支出</td><td className="exp">−¥{evs.filter((e) => e.direction === 'expense').reduce((s, e) => s + Number(e.amount), 0).toFixed(2)}</td></tr>
                <tr><td>调拨出入</td><td>¥{evs.filter((e) => e.direction === 'transfer').reduce((s, e) => s + Number(e.amount), 0).toFixed(2)}（配对内部流转）</td></tr>
                <tr><td><b>当前余额</b></td><td><b>¥{cur.balance.toFixed(2)}</b></td></tr>
                <tr><td>流水笔数</td><td>{cur.event_count} 笔</td></tr>
              </tbody>
            </table>
            <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: 12 }}>
              <button className="primary" onClick={close}>关闭</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'transfers' && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>转账记录</h2>
            <ul className="tasks">
              {tlogs.map((t) => (
                <li key={t.id}>
                  <div>
                    <b>{t.occurred_on} · {t.memo}</b>
                    <div className="hint">对手方：{t.counterparty ?? '—'}</div>
                  </div>
                  <span>¥{t.amount}</span>
                </li>
              ))}
              {!tlogs.length && <li className="hint">暂无转账记录</li>}
            </ul>
            <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
              <button className="primary" onClick={close}>关闭</button>
            </div>
          </div>
        </div>
      )}

      {modal === 'del' && cur && (
        <div className="modal-mask" onClick={close}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <h2>删除账户</h2>
            <p className="hint">确定删除「{cur.name}」？有流水的账户不允许删除（防止账务数据丢失），只能改名。</p>
            <div style={{ display: 'flex', gap: 10, justifyContent: 'flex-end' }}>
              <button className="ghost" onClick={close}>取消</button>
              <button className="primary" style={{ background: 'linear-gradient(135deg,#e2574c,#c2413b)' }} onClick={async () => {
                try {
                  await api.deleteAccount(cur.id);
                  onToast('账户已删除 ✓');
                  setOpenId(null);
                  close();
                  await reload();
                } catch (e) { onToast(String(e)); }
              }}>确认删除</button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}


// ── 侧滑对话面板（数字财务 · 智谱 AskZ 式） ──────────────────

const QUICKS = [
  { title: '税费计算', desc: '这季度要交多少税？', icon: '📊' },
  { title: '用工合规', desc: '300 元临时工怎么处理？', icon: '🧾' },
  { title: '票据疑难', desc: '为什么有的票不能扣？', icon: '📎' },
  { title: '申报提醒', desc: '最近有什么要申报的？', icon: '⏰' },
];

const FAQ_POOL = [
  '这季度要交多少税？',
  '300 元临时工怎么处理？',
  '为什么有的票不能扣？',
  '最近有什么要申报的？',
  '没有发票的支出怎么办？',
  '私车公用油费能报销吗？',
  '股东借钱不还会怎样？',
  '发票抬头开错了怎么办？',
  '季销没超 30 万还要申报吗？',
  '临时工干了 5 天怎么算税？',
];

export function AssistantDrawer({
  persona, apiKey, onClose, onToast,
}: { persona: 'gentle' | 'boss'; apiKey: string; onClose: () => void; onToast: (m: string) => void }) {
  const [msg, setMsg] = useState('');
  const [log, setLog] = useState<{ role: string; text: string }[]>([]);
  const [busy, setBusy] = useState(false);
  const [faq, setFaq] = useState(0); // 换一批翻页
  const boxRef = useRef<HTMLDivElement>(null);
  useEffect(() => { boxRef.current?.scrollTo(0, boxRef.current.scrollHeight); }, [log]);

  const faqItems = Array.from({ length: 4 }, (_, i) => FAQ_POOL[(faq * 4 + i) % FAQ_POOL.length]);

  const ask = async (q: string) => {
    if (!q.trim()) return;
    if (!apiKey) return onToast('点悬浮球右键设置 API Key');
    setMsg('');
    setLog((l) => [...l, { role: 'user', text: q }]);
    setBusy(true);
    try {
      const r = await api.aiChat({ message: q, persona, api_key: apiKey });
      setLog((l) => [...l, { role: 'ai', text: r.answer }]);
    } catch (e) {
      setLog((l) => [...l, { role: 'ai', text: `出错了：${String(e)}` }]);
    } finally { setBusy(false); }
  };

  return (
    <div className="drawer">
      <div className="drawer-head">
        <div />
        <button className="icon-btn" onClick={onClose}>✕</button>
      </div>

      <div className="drawer-body" ref={boxRef}>
        {!log.length && (
          <>
            <div className="hello">
              <div>
                <h2>Hi~</h2>
                <h2>我是数字财务 <span className="wave">✨</span></h2>
                <p className="hint">遇到问题、想知道什么，直接告诉我就好</p>
              </div>
              <Mascot />
            </div>

            <div className="cap-list">
              {QUICKS.map((q) => (
                <button key={q.title} className="cap" onClick={() => ask(q.desc)}>
                  <span className="cap-icon">{q.icon}</span>
                  <div>
                    <b>{q.title}</b>
                    <div className="hint">{q.desc}</div>
                  </div>
                  <span className="cap-arrow">›</span>
                </button>
              ))}
            </div>

            <div className="faq-head">
              <span>您是否想问</span>
              <button className="link-btn" onClick={() => setFaq((f) => f + 1)}>⟳ 换一批</button>
            </div>
            {faqItems.map((q) => (
              <button key={q} className="q-chip" onClick={() => ask(q)}>
                {q} <span className="cap-arrow">›</span>
              </button>
            ))}
            <p className="hint" style={{ marginTop: 12, textAlign: 'center' }}>
              由数字财务提供服务 · 回复仅供参考，重大事项以税务机关口径为准
            </p>
          </>
        )}
        {log.map((m, i) => (
          <div key={i} className={m.role === 'user' ? 'msg me' : 'msg ai'}>{m.text}</div>
        ))}
        {busy && <div className="msg ai">…</div>}
      </div>

      <div className="drawer-input">
        <input
          value={msg}
          onChange={(e) => setMsg(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && ask(msg)}
          placeholder="说说你遇到的情况～"
        />
        <button className="primary" disabled={busy} onClick={() => ask(msg)}>↑</button>
      </div>
    </div>
  );
}

export type { Dashboard };
