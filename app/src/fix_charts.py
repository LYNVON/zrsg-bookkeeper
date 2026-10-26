import re

p = r'C:\Users\93783\XiaomiMiMoProjects\zrsj-bookkeeper\app\src\charts.tsx'
t = open(p, encoding='utf-8').read()

# TrendChart: wrap svg in measured div
t = t.replace(
    "  return (\n    <svg viewBox={`0 0 ${w} ${height}`} style={{ width: '100%', height }}>\n      <defs>\n        <linearGradient id=\"gInc\"",
    "  return (\n    <div ref={ref} style={{ width: '100%' }}>\n    <svg viewBox={`0 0 ${w} ${height}`} width={w} height={height} style={{ width: '100%', height }}>\n      <defs>\n        <linearGradient id=\"gInc\"",
)

idx_bar = t.index('export function BarChart')
seg = t[:idx_bar]
last_svg = seg.rindex('</svg>')
seg = seg[:last_svg] + '</svg>\n    </div>' + seg[last_svg + len('</svg>'):]
t = seg + t[idx_bar:]

# BarChart: hook + wrap
t = t.replace(
    "export function BarChart({ data, height = 150 }: { data: { label: string; income: number; expense: number }[]; height?: number }) {\n  if (!data.length) return <p className=\"hint\">暂无数据</p>;\n  const w = 560;",
    "export function BarChart({ data, height = 150 }: { data: { label: string; income: number; expense: number }[]; height?: number }) {\n  const [ref, w] = useWidth();\n  if (!data.length) return <p className=\"hint\">暂无数据</p>;",
)
t = t.replace(
    "  return (\n    <svg viewBox={`0 0 ${w} ${height}`} style={{ width: '100%', height }}>\n      <defs>\n        <linearGradient id=\"bInc\"",
    "  return (\n    <div ref={ref} style={{ width: '100%' }}>\n    <svg viewBox={`0 0 ${w} ${height}`} width={w} height={height} style={{ width: '100%', height }}>\n      <defs>\n        <linearGradient id=\"bInc\"",
)

idx_pie = t.index('export function PieChart')
seg = t[:idx_pie]
last_svg = seg.rindex('</svg>')
seg = seg[:last_svg] + '</svg>\n    </div>' + seg[last_svg + len('</svg>'):]
t = seg + t[idx_pie:]

open(p, 'w', encoding='utf-8').write(t)
print('charts patched')
