param(
  [Parameter(Mandatory=$true)][string]$File,
  [string]$PfxPath = $env:ZRSJ_SIGN_PFX,
  [string]$PfxPassword = $env:ZRSJ_SIGN_PFX_PASS,
  [string]$Thumbprint = $env:ZRSJ_SIGN_THUMBPRINT,
  [string]$TimestampUrl = "http://timestamp.digicert.com"
)

$ErrorActionPreference = 'Stop'

function Find-SignTool {
  $cands = @(
    "$env:ProgramFiles(x86)\Windows Kits\10\bin\x64\signtool.exe",
    "$env:ProgramFiles\Windows Kits\10\bin\x64\signtool.exe"
  )
  foreach ($c in $cands) { if (Test-Path $c) { return $c } }
  $cmd = Get-Command signtool.exe -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  # 在 SDK 目录里搜一次
  $kits = "$env:ProgramFiles(x86)\Windows Kits\10\bin"
  if (Test-Path $kits) {
    $hit = Get-ChildItem $kits -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
      Where-Object { $_.FullName -match '\\x64\\' } | Select-Object -First 1
    if ($hit) { return $hit.FullName }
  }
  return $null
}

$signTool = Find-SignTool
if (-not $signTool) {
  throw "找不到 signtool.exe（需安装 Windows SDK）。装好后可用 -Thumbprint 或设置 ZRSJ_SIGN_PFX。"
}

if (-not (Test-Path $File)) { throw "待签名文件不存在：$File" }

if ($PfxPath) {
  if (-not (Test-Path $PfxPath)) { throw "证书文件不存在：$PfxPath" }
  Write-Output "使用 PFX 签名：$PfxPath"
  & $signTool sign /fd SHA256 /f $PfxPath /p $PfxPassword /tr $TimestampUrl /td SHA256 $File
} elseif ($Thumbprint) {
  Write-Output "使用证书存储签名（指纹）：$Thumbprint"
  & $signTool sign /fd SHA256 /sha1 $Thumbprint /tr $TimestampUrl /td SHA256 $File
} else {
  throw "未提供证书。请设置 ZRSJ_SIGN_PFX + ZRSJ_SIGN_PFX_PASS，或 ZRSJ_SIGN_THUMBPRINT，再运行本脚本。"
}

if ($LASTEXITCODE -ne 0) { throw "签名失败，退出码 $LASTEXITCODE" }
& $signTool verify /pa $File
if ($LASTEXITCODE -ne 0) { throw "签名校验失败" }
Write-Output "签名完成：$File"
