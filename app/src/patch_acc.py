import re

p = r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\app\src\pages.tsx'
t = open(p, encoding='utf-8').read()

# 1) gear icon at card top-right when open
old_head = '''            <div className="inv-head">
              <b>{a.name}</b>
              <span className={`tag ${a.is_company ? '' : 'warn'}`}>{ACCT_TYPE[a.atype] ?? a.atype} · {a.is_company ? '公司' : '个人'}</span>
            </div>'''
new_head = '''            <div className="inv-head">
              <b>{a.name}</b>
              <div style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
                <span className={`tag ${a.is_company ? '' : 'warn'}`}>{ACCT_TYPE[a.atype] ?? a.atype} · {a.is_company ? '公司' : '个人'}</span>
                {openId === a.id && (
                  <button className="icon-btn" title="账户设置" onClick={(e) => { e.stopPropagation(); setMenuOpen(!menuOpen); }}>⚙</button>
                )}
              </div>
            </div>'''
assert old_head in t, 'head not found'
t = t.replace(old_head, new_head)

# 2) right-click on card opens menu
old_card = '          <div key={a.id} className={openId === a.id ? \'inv-card on\' : \'inv-card\'}>'
new_card = '''          <div key={a.id} className={openId === a.id ? 'inv-card on' : 'inv-card'}
            onContextMenu={(e) => { e.preventDefault(); setOpenId(a.id); setMenuOpen(true); }}>'''
assert old_card in t, 'card not found'
t = t.replace(old_card, new_card)

# 3) simplify acc-quick (drop old gear block)
start = t.index('<div className="acc-quick">')
end = t.index('</div>', t.index('setModal(\'\'); document.getElementById')) + 6
old_block = t[start:end]
new_block = '''<div className="acc-quick">
                  <button className="primary" onClick={() => { setEntry({ direction: 'expense', amount: '', memo: '', category: '', tax_status: '' }); setModal('entry'); }}>＋ 记一笔</button>
                  <button className="ghost" onClick={() => { setTr({ from_id: a.id, to_id: '', amount: '', occurred_on: new Date().toISOString().slice(0, 10), memo: '' }); document.getElementById('acc-transfer')?.scrollIntoView({ behavior: 'smooth' }); }}>转账</button>
                </div>'''
t = t.replace(old_block, new_block)

open(p, 'w', encoding='utf-8').write(t)
print('patched ok')
