// 关联引擎打分（PRD §5.4）——纯函数，无 IO
// 金额一律为字符串 decimal（元，两位小数），内部转 Number 比较（业务量级安全）

const AMT_TOL = 0.01;

export function normName(name) {
  if (!name) return '';
  return String(name)
    .replace(/[（(][^）)]*[）)]/g, '')
    .replace(/(有限公司|股份有限公司|科技|商贸|贸易|咨询|服务|（中国）|\(中国\)|中国)/g, '')
    .replace(/\s+/g, '')
    .toLowerCase();
}

export function nameSimilar(a, b) {
  const na = normName(a), nb = normName(b);
  if (!na || !nb) return false;
  return na === nb || na.includes(nb) || nb.includes(na);
}

export function daysBetween(d1, d2) {
  const a = new Date(d1).getTime(), b = new Date(d2).getTime();
  return Math.abs(a - b) / 86400000;
}

/**
 * 单笔对单笔打分。flow/invoice: {amount, occurred_on|date, counterparty}
 * 金额差 > 0.01 → null（不进候选）
 */
export function scorePair(flow, doc) {
  const dAmt = Math.abs(Number(flow.amount) - Number(doc.amount));
  if (dAmt > AMT_TOL) return null;
  let score = 50;
  if (nameSimilar(flow.counterparty, doc.counterparty)) score += 30;
  const dd = daysBetween(flow.occurred_on ?? flow.date, doc.occurred_on ?? doc.date);
  if (dd <= 3) score += 20;
  if (flow.historyMatch || doc.historyMatch) score += 15;
  return { score, amountDiff: dAmt, daysDiff: dd };
}

/** 批量：给一条流水找候选清单（含 1:N 组合求和，最多 3 笔） */
export function candidates(flow, docs) {
  const out = [];
  for (const d of docs) {
    const s = scorePair(flow, d);
    if (s) out.push({ doc: d, ...s });
  }
  const target = Number(flow.amount);
  const n = docs.length;
  const addCombo = (arr) => {
    const sum = arr.reduce((a, b) => a + Number(b.amount), 0);
    if (Math.abs(sum - target) <= AMT_TOL) {
      out.push({ doc: arr, score: 50, combined: true, amountDiff: Math.abs(sum - target) });
    }
  };
  for (let i = 0; i < n; i++) {
    for (let j = i + 1; j < n; j++) {
      addCombo([docs[i], docs[j]]);
      for (let k = j + 1; k < n; k++) addCombo([docs[i], docs[j], docs[k]]);
    }
  }
  return out.sort((a, b) => b.score - a.score);
}

export const TIER = {
  auto: 80,       // ≥ 自动挂靠
  confirm: 40,    // 40–79 待确认
};

export function tierOf(score) {
  return score >= TIER.auto ? 'auto' : score >= TIER.confirm ? 'confirm' : 'float';
}

/** 银行手续费特例：|差|≤5 且需方向判断由调用方给 */
export function maybeFee(feeDiffAbs) {
  return feeDiffAbs <= 5;
}
