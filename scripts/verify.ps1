# 本地自检：提交前跑一遍，等价于 CI（windows-latest 上的两个 job）
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

function Step($name, [scriptblock]$cmd) {
  Write-Host "`n=== $name ===" -ForegroundColor Cyan
  & $cmd
  if ($LASTEXITCODE -ne 0) { throw "$name 失败（exit $LASTEXITCODE）" }
}

Step '前端类型检查' {
  Push-Location "$root\app"
  try { npx tsc --noEmit } finally { Pop-Location }
}

Step '前端单测' {
  Push-Location "$root\app"
  try { npx vitest run } finally { Pop-Location }
}

Step '后端单测' {
  Push-Location "$root\app\src-tauri"
  try { cargo test --lib } finally { Pop-Location }
}

Write-Host "`n全部通过 ✅" -ForegroundColor Green
