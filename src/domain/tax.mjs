// 税费规则纯函数（PRD §5.7 / §5.13，v1.6 修正口径）
// 参数外置：阈值/税率表可随政策包更新

export const VAT_THRESHOLD = 300000;          // 季销免征线（普票+未开票），专票不免
export const CIT_SMALL_PROFIT_THRESHOLD = 3000000;
export const CIT_SMALL_PROFIT_RATE = 0.05;    // 实际税负（2023 年 6/12 号公告，有效至 2027-12-31）
export const ZERO_SUPPLIER_THRESHOLD_DEFAULT = 500; // 零星按次起征点，可配 300–500

/**
 * 小规模增值税（季）：拆专票/普票/未开票
 * amounts: { special, general, unbilled }（不含税收入额）
 * 返回 { vatPayable, exempt, detail }
 */
export function vatSmallQuarterly({ special = 0, general = 0, unbilled = 0 }, rate = 0.01) {
  const total = special + general + unbilled;
  if (total <= VAT_THRESHOLD) {
    // 免征只覆盖普票+未开票；专票税额不免
    return {
      vatPayable: round2(special * rate),
      exempt: 0,
      detail: {
        threshold: VAT_THRESHOLD,
        total,
        specialTax: round2(special * rate),
        generalExempt: round2(general),
        unbilledExempt: round2(unbilled),
        note: '季销≤30万：普票与未开票免征，专票税额不免',
      },
    };
  }
  return {
    vatPayable: round2((special + general + unbilled) * rate),
    exempt: 0,
    detail: { threshold: VAT_THRESHOLD, total, note: '季销>30万：全额计征' },
  };
}

/** 小微企税实际税负（简化：全年利润） */
export function citSmallProfit(annualProfit) {
  if (annualProfit <= 0) return 0;
  return annualProfit <= CIT_SMALL_PROFIT_THRESHOLD
    ? round2(annualProfit * CIT_SMALL_PROFIT_RATE)
    : round2(annualProfit * 0.25); // 超限按法定粗算，精确表后续参数化
}

/**
 * 劳务报酬预扣（61 号公告三级超额累进）——v1.6 修正，禁止退回简化×20%
 * 应纳税所得额：≤4000 → 收入-800；>4000 → 收入×0.8
 */
export function laborWithhold(income) {
  const inc = Number(income);
  if (inc <= 800) return { taxable: 0, withheld: 0, bracket: 0 };
  const taxable = inc <= 4000 ? inc - 800 : inc * 0.8;
  let rate, quick;
  if (taxable <= 20000) { rate = 0.2; quick = 0; }
  else if (taxable <= 50000) { rate = 0.3; quick = 2000; }
  else { rate = 0.4; quick = 7000; }
  return {
    taxable: round2(taxable),
    rate,
    quick,
    bracket: rate === 0.2 ? 1 : rate === 0.3 ? 2 : 3,
    withheld: round2(taxable * rate - quick),
  };
}

/** 零星小额判定（起征点可配 300–500） */
export function isZeroSmall(amount, threshold = ZERO_SUPPLIER_THRESHOLD_DEFAULT) {
  return Number(amount) <= threshold;
}

function round2(x) {
  return Math.round(x * 100) / 100;
}
