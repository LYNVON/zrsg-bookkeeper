import re

p = r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\app\src\pages.tsx'
t = open(p, encoding='utf-8').read()

start_marker = "{openId === a.id && cur && ("
start = t.index(start_marker)
# 找到任务列表后的收尾：'!evs.length' 后的第一个 '</ul>' 再后面的 ')}'
evs_pos = t.index('{!evs.length && <li className="hint">', start)
ul_end = t.index('</ul>', evs_pos)
close_pos = t.index(')}', ul_end) + 2

clean = '''{openId === a.id && cur && (
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
                            const csv = '日期,方向,金额,对方,事由,分类\\n' + evs.map((e) => `${e.occurred_on},${e.direction},${e.amount},${e.counterparty ?? ''},${e.memo ?? ''},${e.category ?? ''}`).join('\\n');
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
          )}'''

t = t[:start] + clean + t[close_pos:]
open(p, 'w', encoding='utf-8').write(t)
print('repaired ok')
