# 下载/准备内置 OCR 引擎（首次从源码构建时需要执行一次）
# - Tesseract 5.4 Windows x64（UB-Mannheim 构建）
# - chi_sim / eng fast 语言数据
# 结果目录：app/src-tauri/resources/tesseract
param(
  [string]$Root = (Split-Path -Parent $PSScriptRoot),
  [string]$Work = "$env:TEMP\zrsg-tesseract"
)

$ErrorActionPreference = 'Stop'
$dest = Join-Path $Root 'app\src-tauri\resources\tesseract'
$tessdata = Join-Path $dest 'tessdata'
$setup = Join-Path $Work 'tesseract-setup.exe'
$extract = Join-Path $Work 'extract'

New-Item -ItemType Directory -Force -Path $Work, $dest, $tessdata | Out-Null

# 1) 已就绪则跳过
if ((Test-Path (Join-Path $dest 'tesseract.exe')) -and (Test-Path (Join-Path $tessdata 'chi_sim.traineddata'))) {
  Write-Host "已存在：$dest" -ForegroundColor Green
  exit 0
}

# 2) 下载安装包
if (-not (Test-Path $setup)) {
  $url = 'https://github.com/UB-Mannheim/tesseract/releases/download/v5.4.0.20240606/tesseract-ocr-w64-setup-5.4.0.20240606.exe'
  Write-Host "下载 Tesseract：$url"
  Invoke-WebRequest $url -OutFile $setup -TimeoutSec 600
}

# 3) 解包（需要 7-Zip）
$sevenZip = @(
  "$env:ProgramFiles\7-Zip\7z.exe",
  "${env:ProgramFiles(x86)}\7-Zip\7z.exe"
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $sevenZip) { throw '需要 7-Zip 解包安装程序，请先安装 7-Zip（winget install 7zip.7zip）' }

if (-not (Test-Path (Join-Path $extract 'tesseract.exe'))) {
  Write-Host '解包安装程序…'
  & $sevenZip x $setup "-o$extract" -y | Out-Null
}

# 4) 只拷运行必需文件
$deps = @(
  'tesseract.exe','libarchive-13.dll','libb2-1.dll','libbz2-1.dll','libcrypto-3-x64.dll',
  'libdeflate.dll','libexpat-1.dll','libgcc_s_seh-1.dll','libgif-7.dll','libiconv-2.dll',
  'libjbig-0.dll','libjpeg-8.dll','libleptonica-6.dll','libLerc.dll','liblz4.dll',
  'liblzma-5.dll','libopenjp2-7.dll','libpng16-16.dll','libsharpyuv-0.dll','libstdc++-6.dll',
  'libtesseract-5.dll','libtiff-6.dll','libwebp-7.dll','libwebpmux-3.dll','libwinpthread-1.dll',
  'libzstd.dll','zlib1.dll'
)
foreach ($f in $deps) {
  $src = Join-Path $extract $f
  if (Test-Path $src) { Copy-Item $src (Join-Path $dest $f) -Force }
  else { Write-Host "缺少 $f（不同版本可能改名，请检查）" -ForegroundColor Yellow }
}

# 5) 语言数据（fast 版，体积小）
foreach ($lang in 'chi_sim','eng') {
  $target = Join-Path $tessdata "$lang.traineddata"
  if (-not (Test-Path $target)) {
    $u = "https://raw.githubusercontent.com/tesseract-ocr/tessdata_fast/main/$lang.traineddata"
    Write-Host "下载 $lang.traineddata"
    Invoke-WebRequest $u -OutFile $target -TimeoutSec 600
  }
}

$size = (Get-ChildItem $dest -Recurse -File | Measure-Object Length -Sum).Sum / 1MB
Write-Host ("完成：{0}（{1:N1} MB）" -f $dest, $size) -ForegroundColor Green
