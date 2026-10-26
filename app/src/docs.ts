// 文书/单据 HTML 生成器（打印 → 浏览器另存 PDF）
// 样式对齐 PRD §5.15/§5.11：变量填充、大写金额、打印友好

const STYLE = `
  body{font-family:'SimSun','Songti SC',serif;color:#111;margin:24px;}
  h1{text-align:center;font-size:22px;letter-spacing:8px;margin:6px 0 18px;}
  .meta{display:flex;justify-content:space-between;font-size:13px;margin-bottom:10px;}
  table{width:100%;border-collapse:collapse;font-size:13px;}
  th,td{border:1px solid #444;padding:7px 8px;}
  th{background:#f0f0f0;font-weight:600;}
  .total{margin-top:12px;font-size:13px;}
  .sign{margin-top:34px;display:flex;justify-content:space-between;font-size:13px;}
  .body{font-size:14px;line-height:2;}
  .tip{margin-top:22px;font-size:11px;color:#666;border-top:1px dashed #999;padding-top:8px;}
  @media print{body{margin:12mm;}}
`;

export function shell(title: string, body: string): string {
  return `<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><title>${title}</title><style>${STYLE}</style></head><body>${body}</body></html>`;
}

export const cnUpper = (n: number): string => {
  // 简版大写金额（元角分）
  const digits = '零壹贰叁肆伍陆柒捌玖';
  const units = ['', '拾', '佰', '仟'];
  const bigs = ['', '万', '亿'];
  const toInt = Math.floor(Math.abs(n));
  const jiao = Math.floor(Math.abs(n) * 10) % 10;
  const fen = Math.round(Math.abs(n) * 100) % 10;
  if (toInt === 0 && jiao === 0 && fen === 0) return '人民币零元整';
  let s = '';
  let x = toInt;
  let bigIdx = 0;
  if (x === 0) s = '零';
  while (x > 0) {
    const section = x % 10000;
    let sec = '';
    let y = section;
    let u = 0;
    if (section === 0) {
      sec = '零';
    } else {
      while (y > 0) {
        const d = y % 10;
        if (d !== 0) sec = digits[d] + units[u] + sec;
        else if (sec && !sec.startsWith('零')) sec = '零' + sec;
        y = Math.floor(y / 10);
        u++;
      }
    }
    s = sec + (section > 0 ? bigs[bigIdx] : '') + s;
    x = Math.floor(x / 10000);
    bigIdx++;
  }
  s = s.replace(/零+/g, '零').replace(/零$/, '') + '元';
  if (jiao === 0 && fen === 0) s += '整';
  else {
    if (jiao > 0) s += digits[jiao] + '角';
    else if (fen > 0) s += '零';
    if (fen > 0) s += digits[fen] + '分';
  }
  return '人民币' + s;
};

export interface RbLine {
  expense_on: string;
  category: string;
  invoice_no: string | null;
  amount: number;
}

/** 报销单（对齐用户 QQ 邮箱截图样式） */
export function renderReimbursement(opt: {
  title: string;
  applicant: string;
  formDate: string;
  lines: RbLine[];
}): string {
  const total = opt.lines.reduce((a, l) => a + l.amount, 0);
  // 按费用类型聚合
  const byCat = new Map<string, RbLine[]>();
  for (const l of opt.lines) {
    if (!byCat.has(l.category)) byCat.set(l.category, []);
    byCat.get(l.category)!.push(l);
  }
  let idx = 0;
  let rows = '';
  for (const [cat, ls] of byCat) {
    idx++;
    const sum = ls.reduce((a, l) => a + l.amount, 0);
    rows += `<tr><td>${idx}</td><td>${cat}</td><td>${ls.length}</td><td>${sum.toFixed(2)}</td></tr>`;
    for (const l of ls) {
      rows += `<tr><td></td><td>　└ ${l.invoice_no ?? '(无号)'}</td><td>${l.expense_on}</td><td>${l.amount.toFixed(2)}</td></tr>`;
    }
  }
  return shell(
    '报销单',
    `<h1>报 销 单</h1>
     <div class="meta"><span>报销事项：${opt.title || '日常费用报销'}</span><span>报销人：${opt.applicant || '　'}</span></div>
     <table>
       <tr><th>费用类型序号</th><th>费用类型</th><th>发票数量</th><th>金额（元）</th></tr>
       ${rows}
       <tr><td colspan="2"><b>合计</b></td><td><b>${opt.lines.length}</b></td><td><b>${total.toFixed(2)}</b></td></tr>
     </table>
     <div class="total"><b>金额（大写）：</b>${cnUpper(total)}</div>
     <div class="sign"><span>审批人签字：__________</span><span>日期：${opt.formDate}</span></div>
     <div class="tip">自然拾光记账 · 生成的报销单为参考格式，重大事项请咨询专业人士。</div>`,
  );
}

/** 零星小额劳务收款凭证（≤500，税前可扣） */
export function renderReceipt(opt: {
  name: string;
  idCard: string;
  reason: string;
  amount: number;
  date: string;
  company: string;
}): string {
  return shell(
    '收款收据',
    `<h1>收 款 收 据</h1>
     <div class="body">
       <p>今收到 <b>${opt.company}</b> 支付的：</p>
       <p>事由（项目/工时）：${opt.reason}</p>
       <p>金额（大写）：${cnUpper(opt.amount)}　（小写）¥${opt.amount.toFixed(2)}</p>
       <p>收款人姓名：${opt.name}　身份证号：${opt.idCard}</p>
       <p>日期：${opt.date}</p>
       <p style="margin-top:28px">收款人签字：____________</p>
     </div>
     <div class="tip">依据：国家税务总局公告 2018 年第 28 号（零星小额业务凭收款凭证税前扣除）。支付方为个税扣缴义务人，请勿遗漏申报。</div>`,
  );
}

/** 短期劳务协议 */
export function renderLaborContract(opt: {
  company: string;
  taxNo: string;
  worker: string;
  idCard: string;
  title: string;
  days: string;
  total: number;
  date: string;
}): string {
  return shell(
    '短期劳务协议',
    `<h1>短期劳务协议</h1>
     <div class="body">
       <p>甲方（用工方）：${opt.company}（税号：${opt.taxNo}）</p>
       <p>乙方（提供劳务方）：${opt.worker}（身份证号：${opt.idCard}）</p>
       <p>一、劳务内容：${opt.title}</p>
       <p>二、劳务期间：${opt.days}</p>
       <p>三、劳务报酬：合计人民币 ${opt.total.toFixed(2)} 元（大写：${cnUpper(opt.total)}），按完成进度结算。</p>
       <p>四、双方确认为劳务合作关系，非劳动关系；乙方自行承担其个人所得税申报义务的配合（甲方依法履行扣缴义务）。</p>
       <p>五、本协议一式两份，双方各执一份。</p>
     </div>
     <div class="sign"><span>甲方签字/盖章：__________</span><span>乙方签字：__________</span></div>
     <div class="sign"><span>日期：${opt.date}</span></div>
     <div class="tip">参考范本，重大事项建议咨询专业人士。</div>`,
  );
}

/** 劳务结算单 */
export function renderSettlement(opt: {
  company: string;
  worker: string;
  title: string;
  total: number;
  paid: number;
  tax: number;
  date: string;
}): string {
  return shell(
    '劳务结算单',
    `<h1>劳 务 结 算 单</h1>
     <div class="body">
       <p>用工方：${opt.company}　　劳务方：${opt.worker}</p>
       <p>事项（工单）：${opt.title}</p>
       <table>
         <tr><th>项目</th><th>金额（元）</th></tr>
         <tr><td>约定报酬总额</td><td>${opt.total.toFixed(2)}</td></tr>
         <tr><td>已支付</td><td>${opt.paid.toFixed(2)}</td></tr>
         <tr><td>本次应付</td><td>${(opt.total - opt.paid).toFixed(2)}</td></tr>
         <tr><td>代扣个人所得税（劳务报酬预扣）</td><td>${opt.tax.toFixed(2)}</td></tr>
       </table>
       <p>结算说明：以上按一次劳务报酬结算，甲方依法预扣预缴个人所得税。</p>
     </div>
     <div class="sign"><span>甲方确认：__________</span><span>乙方确认：__________</span></div>
     <div class="sign"><span>日期：${opt.date}</span></div>
     <div class="tip">预扣计算依据：国家税务总局公告 2018 年第 61 号（三级超额累进）。</div>`,
  );
}

/** 私车公用协议 */
export function renderCarAgreement(opt: {
  company: string;
  taxNo: string;
  owner: string;
  plate: string;
  idCard: string;
  date: string;
}): string {
  return shell(
    '私车公用协议',
    `<h1>私 车 公 用 协 议</h1>
     <div class="body">
       <p>甲方（公司）：${opt.company}（税号：${opt.taxNo}）</p>
       <p>乙方（车主）：${opt.owner}（身份证号：${opt.idCard}）</p>
       <p>一、乙方将其自有车辆（车牌号：${opt.plate}）用于甲方公务活动。</p>
       <p>二、公务使用期间产生的合理费用（燃油费、过路费、停车费）凭合法有效发票由甲方实报实销，发票抬头开甲方全称及税号。</p>
       <p>三、车辆所有权、保险、维修保养、违章责任由乙方自行承担；非公务使用费用甲方不予承担。</p>
       <p>四、本协议自双方签字之日起生效，有效期至双方另行协商终止。</p>
     </div>
     <div class="sign"><span>甲方（盖章）：__________</span><span>乙方签字：__________</span></div>
     <div class="sign"><span>日期：${opt.date}</span></div>
     <div class="tip">参考范本，重大事项建议咨询专业人士。油费报销请连同本协议编号归档。</div>`,
  );
}

/** 借款借据 */
export function renderLoanNote(opt: {
  company: string;
  borrower: string;
  idCard: string;
  amount: number;
  dueDate: string;
  purpose: string;
  date: string;
}): string {
  return shell(
    '借款借据',
    `<h1>借 据</h1>
     <div class="body">
       <p>今借到 ${opt.company} 人民币：</p>
       <p>金额（大写）：${cnUpper(opt.amount)}　（小写）¥${opt.amount.toFixed(2)}</p>
       <p>借款用途：${opt.purpose}</p>
       <p>借款人：${opt.borrower}（身份证号：${opt.idCard}）</p>
       <p>约定归还日期：${opt.dueDate}（<b>请勿跨会计年度未还，否则存在视同分红风险</b>）</p>
       <p style="margin-top:28px">借款人签字：____________　　日期：${opt.date}</p>
     </div>
     <div class="tip">依据：财税〔2003〕158 号——股东借款跨会计年度未还，可能被视同分红计征个税。</div>`,
  );
}

/** 劳务报酬预扣（61 号公告三级超额累进，对齐 domain/tax.mjs） */
export function laborWithhold(income: number): { taxable: number; rate: number; quick: number; withheld: number } {
  if (income <= 800) return { taxable: 0, rate: 0, quick: 0, withheld: 0 };
  const taxable = income <= 4000 ? income - 800 : income * 0.8;
  let rate = 0.2,
    quick = 0;
  if (taxable > 50000) {
    rate = 0.4;
    quick = 7000;
  } else if (taxable > 20000) {
    rate = 0.3;
    quick = 2000;
  }
  return {
    taxable: Math.round(taxable * 100) / 100,
    rate,
    quick,
    withheld: Math.round((taxable * rate - quick) * 100) / 100,
  };
}
