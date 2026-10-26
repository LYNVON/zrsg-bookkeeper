// SVG 图表（v2.2）：实测容器宽度铺满 · 12 个月完整轴 · 正常 Y 轴分级 · 收支分色环形图

import { useEffect, useRef, useState } from 'react';

const INK = '#98a1b3';
const GRID = '#eef1f6';

/** 实测容器宽度（ResizeObserver）→ 图表按真实像素绘制，随卡片拉伸 */
function useWidth() {
  const ref = useRef<HTMLDivElement>(null);
  const [w, setW] = useState(560);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const apply = (cw: number) => { if (cw > 40) setW(cw); };
    apply(el.clientWidth);
    const ro = new ResizeObserver((es) => apply(es[0].contentRect.width));
    ro.observe(el);
    return () => ro.disconnect();
  }, []);
  return [ref, w] as const;
}

/** 数字标签：千/万缩写 */
const fmt = (n: number) => {
  const v = Math.round(n);
  if (Math.abs(v) >= 10000) return `${(v / 10000).toFixed(v % 10000 === 0 ? 0 : 1)}万`;
  if (Math.abs(v) >= 1000) return `${(v / 1000).toFixed(v % 1000 === 0 ? 0 : 1)}千`;
  return String(v);
};

/** 优美的 Y 轴刻度：0 起步、1/2/5×10^n 分级 */
function niceScale(rawMax: number): { max: number; ticks: number[] } {
  const target = Math.max(rawMax, 1);
  const rough = target / 4;
  const pow = Math.pow(10, Math.floor(Math.log10(rough)));
  const mult = rough / pow;
  const step = (mult <= 1 ? 1 : mult <= 2 ? 2 : mult <= 5 ? 5 : 10) * pow;
  const max = Math.ceil(target / step) * step;
  const ticks: number[] = [];
  for (let v = 0; v <= max + 1e-9; v += step) ticks.push(v);
  return { max, ticks };
}

/** 月份标签：1月…12月，跨年在 1 月前加年份 */
function monthLabel(key: string, isJan: boolean): string {
  const m = key.slice(5).replace(/^0/, '');
  return isJan ? `${key.slice(2, 4)}年${m}月` : `${m}月`;
}

function smoothPath(pts: { x: number; y: number }[]): string {
  if (pts.length === 0) return '';
  if (pts.length === 1) return `M${pts[0].x},${pts[0].y}`;
  let d = `M${pts[0].x},${pts[0].y}`;
  for (let i = 0; i < pts.length - 1; i++) {
    const p0 = pts[i === 0 ? 0 : i - 1];
    const p1 = pts[i];
    const p2 = pts[i + 1];
    const p3 = pts[i + 2 < pts.length ? i + 2 : i + 1];
    d += ` C${p1.x + (p2.x - p0.x) / 6},${p1.y + (p2.y - p0.y) / 6} ${p2.x - (p3.x - p1.x) / 6},${p2.y - (p3.y - p1.y) / 6} ${p2.x},${p2.y}`;
  }
  return d;
}

export function TrendChart({ data, height = 168 }: { data: { label: string; income: number; expense: number }[]; height?: number }) {
  const [ref, w] = useWidth();
  if (!data.length) return <p className="hint">暂无数据</p>;
  const padL = 42, padR = 14, padT = 14, padB = 28;
  const { max, ticks } = niceScale(Math.max(1, ...data.map((d) => Math.max(d.income, d.expense))));
  const step = (w - padL - padR) / Math.max(1, data.length - 1);
  const y = (v: number) => height - padB - (v / max) * (height - padT - padB);
  const pts = (k: 'income' | 'expense') => data.map((d, i) => ({ x: padL + i * step, y: y(d[k]) }));
  const inc = pts('income'), exp = pts('expense');
  const area = (p: { x: number; y: number }[]) =>
    `${smoothPath(p)} L${p[p.length - 1].x},${height - padB} L${p[0].x},${height - padB} Z`;

  return (
    <div ref={ref} style={{ width: '100%' }}>
    <svg viewBox={`0 0 ${w} ${height}`} width={w} height={height} style={{ width: '100%', height }}>
      <defs>
        <linearGradient id="gInc" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#3ba776" stopOpacity="0.2" />
          <stop offset="100%" stopColor="#3ba776" stopOpacity="0" />
        </linearGradient>
        <linearGradient id="gExp" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#e2706b" stopOpacity="0.16" />
          <stop offset="100%" stopColor="#e2706b" stopOpacity="0" />
        </linearGradient>
        <linearGradient id="lInc" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#6ee7b7" /><stop offset="100%" stopColor="#2f9e6f" />
        </linearGradient>
        <linearGradient id="lExp" x1="0" y1="0" x2="1" y2="0">
          <stop offset="0%" stopColor="#f7a8a2" /><stop offset="100%" stopColor="#d9534c" />
        </linearGradient>
      </defs>

      {ticks.map((t) => (
        <g key={t}>
          <line x1={padL} x2={w - padR} y1={y(t)} y2={y(t)} stroke={GRID} strokeWidth="1" strokeDasharray="3 5" />
          <text x={padL - 8} y={y(t)} textAnchor="end" dominantBaseline="middle" fontSize="9" fill={INK}>{fmt(t)}</text>
        </g>
      ))}

      <path d={area(inc)} fill="url(#gInc)" />
      <path d={area(exp)} fill="url(#gExp)" />
      <path d={smoothPath(inc)} fill="none" stroke="url(#lInc)" strokeWidth="2" strokeLinecap="round" />
      <path d={smoothPath(exp)} fill="none" stroke="url(#lExp)" strokeWidth="2" strokeLinecap="round" />

      {data.map((d, i) => (
        <g key={d.label}>
          {(d.income > 0 || data.length <= 2) && <circle cx={inc[i].x} cy={inc[i].y} r="2.6" fill="#fff" stroke="#3ba776" strokeWidth="1.4" />}
          {(d.expense > 0 || data.length <= 2) && <circle cx={exp[i].x} cy={exp[i].y} r="2.6" fill="#fff" stroke="#e2706b" strokeWidth="1.4" />}
          <text x={padL + i * step} y={height - 8} textAnchor="middle" fontSize="9" fill={INK}>
            {monthLabel(d.label, d.label.slice(5) === '01' || i === 0)}
          </text>
        </g>
      ))}
    </svg>
    </div>
  );
}

export function BarChart({ data, height = 150 }: { data: { label: string; income: number; expense: number }[]; height?: number }) {
  const [ref, w] = useWidth();
  if (!data.length) return <p className="hint">暂无数据</p>;
  const padL = 36, padR = 12, padT = 12, padB = 26;
  const { max, ticks } = niceScale(Math.max(1, ...data.map((d) => Math.max(d.income, d.expense))));
  const slot = (w - padL - padR) / data.length;
  const bw = Math.min(13, slot * 0.22);
  const y = (v: number) => height - padB - (v / max) * (height - padT - padB);
  return (
    <div ref={ref} style={{ width: '100%' }}>
    <svg viewBox={`0 0 ${w} ${height}`} width={w} height={height} style={{ width: '100%', height }}>
      <defs>
        <linearGradient id="bInc" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#5fc798" /><stop offset="100%" stopColor="#2f9e6f" stopOpacity="0.55" />
        </linearGradient>
        <linearGradient id="bExp" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0%" stopColor="#f09b95" /><stop offset="100%" stopColor="#d9534c" stopOpacity="0.55" />
        </linearGradient>
      </defs>
      {ticks.map((t) => (
        <g key={t}>
          <line x1={padL} x2={w - padR} y1={y(t)} y2={y(t)} stroke={GRID} strokeWidth="1" strokeDasharray="3 5" />
          <text x={padL - 6} y={y(t)} textAnchor="end" dominantBaseline="middle" fontSize="9" fill={INK}>{fmt(t)}</text>
        </g>
      ))}
      {data.map((d, i) => {
        const cx = padL + slot * (i + 0.5);
        const hi = (d.income / max) * (height - padT - padB);
        const he = (d.expense / max) * (height - padT - padB);
        return (
          <g key={d.label}>
            <rect x={cx - bw - 2} y={height - padB - hi} width={bw} height={Math.max(hi, d.income > 0 ? 2 : 0)} fill="url(#bInc)" rx={bw / 2} />
            <rect x={cx + 2} y={height - padB - he} width={bw} height={Math.max(he, d.expense > 0 ? 2 : 0)} fill="url(#bExp)" rx={bw / 2} />
            <text x={cx} y={height - 8} textAnchor="middle" fontSize="9" fill={INK}>
              {monthLabel(d.label, d.label.slice(5) === '01' || i === 0)}
            </text>
          </g>
        );
      })}
    </svg>
    </div>
  );
}

export function PieChart({ data, size = 150, title = '支出', variant = 'expense' }: { data: { label: string; value: number }[]; size?: number; title?: string; variant?: 'income' | 'expense' }) {
  const total = data.reduce((a, d) => a + d.value, 0) || 1;
  // 收支分色：支出=暖色系（红/粉/橙），收入=冷色系（绿/青/蓝）
  const colors = variant === 'expense'
    ? ['#e2706b', '#f09b95', '#f0a86b', '#f48fb1', '#d98a6a', '#e8b4bc', '#c96b6b', '#f2c4a0']
    : ['#2f9e6f', '#5fc798', '#4dd0c7', '#60a5fa', '#7cc9a8', '#38b2ac', '#8b7cf6', '#94c9a8'];
  const cx = size / 2, cy = size / 2;
  const rOut = size / 2 - 6;
  const rIn = rOut * 0.62;
  let angle = -Math.PI / 2;
  const gap = 0.02;
  const slices = data.map((d, i) => {
    const frac = d.value / total;
    const a2 = angle + frac * Math.PI * 2;
    const a1 = angle + gap / 2;
    const aEnd = Math.max(a1 + 0.001, a2 - gap / 2);
    const large = aEnd - a1 > Math.PI ? 1 : 0;
    const path = `M ${cx + rIn * Math.cos(a1)} ${cy + rIn * Math.sin(a1)} L ${cx + rOut * Math.cos(a1)} ${cy + rOut * Math.sin(a1)} A ${rOut} ${rOut} 0 ${large} 1 ${cx + rOut * Math.cos(aEnd)} ${cy + rOut * Math.sin(aEnd)} L ${cx + rIn * Math.cos(aEnd)} ${cy + rIn * Math.sin(aEnd)} A ${rIn} ${rIn} 0 ${large} 0 ${cx + rIn * Math.cos(a1)} ${cy + rIn * Math.sin(a1)} Z`.replace(/\s+/g, ' ');
    angle = a2;
    return { path, color: colors[i % colors.length], d };
  });
  return (
    <div style={{ display: 'flex', gap: 18, alignItems: 'center' }}>
      <svg viewBox={`0 0 ${size} ${size}`} style={{ width: size, height: size, flexShrink: 0 }}>
        {slices.map((s, i) => <path key={i} d={s.path} fill={s.color} opacity={0.92} />)}
        <text x={cx} y={cy - 4} textAnchor="middle" fontSize="11" fill={INK}>{title}</text>
        <text x={cx} y={cy + 12} textAnchor="middle" fontSize="13" fontWeight="650" fill="#1a2233">{fmt(total)}</text>
      </svg>
      <ul style={{ listStyle: 'none', margin: 0, padding: 0, fontSize: 11.5, display: 'flex', flexDirection: 'column', gap: 6 }}>
        {slices.map((s, i) => (
          <li key={i} style={{ display: 'flex', gap: 8, alignItems: 'center' }}>
            <span style={{ width: 8, height: 8, borderRadius: '50%', background: s.color, flexShrink: 0 }} />
            <span style={{ color: '#5b6577' }}>{s.d.label}</span>
            <b style={{ marginLeft: 'auto', fontVariantNumeric: 'tabular-nums' }}>{((s.d.value / total) * 100).toFixed(0)}%</b>
          </li>
        ))}
      </ul>
    </div>
  );
}
