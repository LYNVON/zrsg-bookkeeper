param(
  [Parameter(Mandatory=$true)][string]$Version,
  [Parameter(Mandatory=$true)][string]$Repo,          # owner/repo
  [string]$Notes = "",
  [switch]$NoPush
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$app = Join-Path $root 'app'
$confPath = Join-Path $app 'src-tauri\tauri.conf.json'
$pkgPath = Join-Path $app 'package.json'
$keyPath = Join-Path $root 'keys\updater.key'

if (-not (Test-Path $keyPath)) { throw "缺少更新签名私钥：$keyPath" }
if ($Version -notmatch '^\d+\.\d+\.\d+$') { throw "版本号格式应为 x.y.z" }

# 1) 版本号 + 更新源写入配置
$conf = Get-Content $confPath -Raw -Encoding UTF8 | ConvertFrom-Json
$old = $conf.version
$conf.version = $Version
$conf.plugins.updater.endpoints = @("https://github.com/$Repo/releases/latest/download/latest.json")
$conf | ConvertTo-Json -Depth 20 | Set-Content $confPath -Encoding UTF8

$pkg = Get-Content $pkgPath -Raw -Encoding UTF8 | ConvertFrom-Json
$pkg.version = $Version
$pkg | ConvertTo-Json -Depth 20 | Set-Content $pkgPath -Encoding UTF8
Write-Host "版本 $old -> $Version ; 更新源 -> github.com/$Repo" -ForegroundColor Cyan

# 2) 构建（带更新签名）
$env:TAURI_SIGNING_PRIVATE_KEY = $keyPath
if ($env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD -eq $null) {
  Write-Host "未设置 TAURI_SIGNING_PRIVATE_KEY_PASSWORD（若私钥有密码会失败）" -ForegroundColor Yellow
}
Push-Location $app
try {
  npx tauri build
  if ($LASTEXITCODE -ne 0) { throw "构建失败（exit $LASTEXITCODE）" }
} finally { Pop-Location }

# 3) 收集产物
$nsisDir = Join-Path $app 'src-tauri\target\release\bundle\nsis'
$installer = Get-ChildItem $nsisDir -Filter '*setup.exe' | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $installer) { throw "找不到 NSIS 安装包：$nsisDir" }
$sigPath = "$($installer.FullName).sig"
if (-not (Test-Path $sigPath)) { throw "找不到签名文件：$sigPath（检查 createUpdaterArtifacts / 私钥）" }

$tag = "v$Version"
$assetName = $installer.Name
$assetUrl = "https://github.com/$Repo/releases/download/$tag/$([uri]::EscapeDataString($assetName))"
$pubDate = (Get-Date).ToUniversalTime().ToString("yyyy-MM-ddTHH:mm:ssZ")

$latest = [ordered]@{
  version   = $Version
  notes     = if ($Notes) { $Notes } else { "自然拾光记账 $Version" }
  pub_date  = $pubDate
  platforms = [ordered]@{
    'windows-x86_64' = [ordered]@{
      signature = (Get-Content $sigPath -Raw).Trim()
      url       = $assetUrl
    }
  }
}
$latestPath = Join-Path $nsisDir 'latest.json'
$latest | ConvertTo-Json -Depth 10 | Set-Content $latestPath -Encoding UTF8
Write-Host "latest.json 已生成：$latestPath" -ForegroundColor Cyan

# 4) 提交并发版
Push-Location $root
try {
  git add -A
  git -c user.name="$env:USERNAME" -c user.email="$env:USERNAME@users.noreply.github.com" commit -m "release: v$Version" | Out-Null
  if (-not $NoPush) {
    git push -u origin HEAD
    gh release create $tag $installer.FullName $sigPath $latestPath --repo $Repo --title "v$Version" --notes $latest.notes
  }
} finally { Pop-Location }

Write-Host "`n完成：$tag 已发布到 $Repo" -ForegroundColor Green
