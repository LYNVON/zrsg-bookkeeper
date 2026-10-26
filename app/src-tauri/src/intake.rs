// 票夹收票：本地附件导入（归档 + 解析 + 自动归类）/ 邮箱配置持久化 / 文件选择
use crate::db::{data_dir, new_id, DbState};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Serialize, Clone)]
pub struct ImportResult {
    pub file: String,
    pub ok: bool,
    pub duplicate: bool,
    pub invoice_no: Option<String>,
    pub amount: Option<String>,
    pub issued_on: Option<String>,
    pub seller: Option<String>,
    pub category: Option<String>,
    pub message: String,
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn sha256_file(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    hex(&h.finalize())
}

/// 20 位数电票号（以 2 开头）
fn find_invoice_no(text: &str) -> Option<String> {
    let b = text.as_bytes();
    let mut i = 0;
    while i + 20 <= b.len() {
        if b[i..i + 20].iter().all(|c| c.is_ascii_digit()) {
            let cand = &text[i..i + 20];
            if cand.starts_with('2') {
                return Some(cand.to_string());
            }
            i += 20;
        } else {
            i += 1;
        }
    }
    None
}

/// 价税合计金额
fn find_amount(text: &str) -> Option<String> {
    for key in ["价税合计", "小写", "合计", "TOTAL", "Total", "total", "AMOUNT", "Amount", "合计金额"] {
        if let Some(pos) = text.find(key) {
            let tail: String = text[pos..].chars().take(40).collect();
            let mut num = String::new();
            let mut started = false;
            for c in tail.chars() {
                if c.is_ascii_digit() || c == '.' {
                    num.push(c);
                    started = true;
                } else if started {
                    break;
                }
            }
            let num = num.trim_matches('.').to_string();
            if let Ok(v) = num.parse::<f64>() {
                if v > 0.0 && v < 100_000_000.0 {
                    return Some(format!("{v:.2}"));
                }
            }
        }
    }
    None
}

/// 开票日期：2026年10月26日 / 2026-10-26
fn find_date(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i + 6 < chars.len() {
        if chars[i] == '2' && chars[i + 1] == '0' {
            // YYYY年M月D日
            let seg: String = chars[i..(i + 12).min(chars.len())].iter().collect();
            if let Some(y_end) = seg.find('年') {
                let y: String = seg[..y_end].chars().filter(|c| c.is_ascii_digit()).collect();
                let rest = &seg[y_end + '年'.len_utf8()..];
                if let Some(m_end) = rest.find('月') {
                    let m: String = rest[..m_end].chars().filter(|c| c.is_ascii_digit()).collect();
                    let rest2 = &rest[m_end + '月'.len_utf8()..];
                    if let Some(d_end) = rest2.find('日') {
                        let d: String = rest2[..d_end].chars().filter(|c| c.is_ascii_digit()).collect();
                        if y.len() == 4 && !m.is_empty() && !d.is_empty() {
                            let (m, d): (u32, u32) = (m.parse().ok()?, d.parse().ok()?);
                            if (1..=12).contains(&m) && (1..=31).contains(&d) {
                                return Some(format!("{y}-{m:02}-{d:02}"));
                            }
                        }
                    }
                }
            }
            // YYYY-MM-DD（允许中间夹空格）
            let seg2: String = chars[i..(i + 10).min(chars.len())].iter().collect();
            let parts: Vec<String> = seg2
                .split(|c: char| c == '-' || c == '/' || c == ' ')
                .map(|s| s.chars().filter(|c| c.is_ascii_digit()).collect())
                .collect();
            if parts.len() >= 3 && parts[0].len() == 4 && !parts[1].is_empty() && !parts[2].is_empty() {
                if let (Ok(m), Ok(d)) = (parts[1].parse::<u32>(), parts[2].parse::<u32>()) {
                    if (1..=12).contains(&m) && (1..=31).contains(&d) {
                        return Some(format!("{}-{m:02}-{d:02}", parts[0]));
                    }
                }
            }
        }
        i += 1;
    }
    None
}

/// 销方 / 购方：取 "名称" 后的中文公司名，结合公司档案判断哪边是购方
fn find_parties(text: &str, company: Option<&str>) -> (Option<String>, Option<String>) {
    let mut names: Vec<String> = Vec::new();
    for (idx, _) in text.match_indices("名称") {
        let tail: String = text[idx..].chars().take(40).collect();
        let cleaned: String = tail
            .trim_start_matches("名称")
            .trim_start_matches(|c: char| c == '：' || c == ':' || c == ' ' || c == '\n' || c == '\t')
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != '\n' && *c != '：' && *c != ':')
            .collect();
        if cleaned.chars().count() >= 4 && !names.contains(&cleaned) {
            names.push(cleaned);
        }
    }
    let is_buyer = |n: &str| company.map(|c| n.contains(c) || c.contains(n)).unwrap_or(false);
    let buyer = names.iter().find(|n| is_buyer(n)).cloned();
    let seller = names.iter().find(|n| !is_buyer(n)).cloned();
    (seller, buyer)
}

/// 按销方/文本关键词自动归类
pub fn categorize(seller: Option<&str>, text: &str) -> String {
    let hay = format!("{} {}", seller.unwrap_or(""), text);
    let rules: [(&[&str], &str); 8] = [
        (&["餐饮", "餐费", "food", "catering", "coffee", "咖啡", "茶", "餐厅"], "餐饮"),
        (&["加油", "停车", "高速", "出租车", "滴滴", "打车", "航空", "机票", "铁路", "运输", "交通"], "交通"),
        (&["酒店", "住宿", "宾馆", "民宿"], "差旅"),
        (&["办公", "文具", "耗材", "纸张", "打印", "电脑", "软件", "服务费", "IT"], "办公"),
        (&["房租", "物业", "水电", "供暖", "宽带", "通信", "话费"], "房租水电"),
        (&["咨询", "广告", "推广", "设计", "代理", "律师", "会计", "审计"], "服务"),
        (&["建材", "五金", "材料", "设备", "维修"], "采购"),
        (&["快递", "物流", "仓储"], "物流"),
    ];
    let low = hay.to_lowercase();
    for (keys, cat) in rules {
        if keys.iter().any(|k| low.contains(&k.to_lowercase())) {
            return cat.to_string();
        }
    }
    "其他".to_string()
}


// ── OCR（Windows 自带 zh-Hans-CN 引擎，无需联网/额外依赖）─────────

const OCR_PS: &str = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
function Await($op, $type) {
    $task = $asTaskGeneric.MakeGenericMethod($type).Invoke($null, @($op))
    $task.Wait(-1) | Out-Null
    $task.Result
}
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Media.Ocr.OcrEngine, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Graphics.Imaging.BitmapDecoder, Windows.Foundation, ContentType = WindowsRuntime]
$null = [Windows.Globalization.Language, Windows.Foundation, ContentType = WindowsRuntime]
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($env:ZRSJ_OCR_IN)) ([Windows.Storage.StorageFile])
$stream = Await ($file.OpenAsync([Windows.Storage.FileAccessMode]::Read)) ([Windows.Storage.Streams.IRandomAccessStream])
$decoder = Await ([Windows.Graphics.Imaging.BitmapDecoder]::CreateAsync($stream)) ([Windows.Graphics.Imaging.BitmapDecoder])
$bitmap = Await ($decoder.GetSoftwareBitmapAsync()) ([Windows.Graphics.Imaging.SoftwareBitmap])
$engine = $null
try {
    $lang = New-Object Windows.Globalization.Language 'zh-Hans-CN'
    $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromLanguage($lang)
} catch { }
if (-not $engine) { $engine = [Windows.Media.Ocr.OcrEngine]::TryCreateFromUserProfileLanguages() }
if (-not $engine) { exit 0 }
$result = Await ($engine.RecognizeAsync($bitmap)) ([Windows.Media.Ocr.OcrResult])
$result.Lines | ForEach-Object { $_.Text }
"#;

const PDF2PNG_PS: &str = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName System.Runtime.WindowsRuntime | Out-Null
$asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]
$asTaskAction = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {
    $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and
    $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncAction' })[0]
function Await($op, $type) {
    $task = $asTaskGeneric.MakeGenericMethod($type).Invoke($null, @($op))
    $task.Wait(-1) | Out-Null
    $task.Result
}
function AwaitAction($op) {
    $task = $asTaskAction.Invoke($null, @($op))
    $task.Wait(-1) | Out-Null
}
$null = [Windows.Storage.StorageFile, Windows.Storage, ContentType = WindowsRuntime]
$null = [Windows.Data.Pdf.PdfDocument, Windows.Data.Pdf, ContentType = WindowsRuntime]
$null = [Windows.Storage.Streams.InMemoryRandomAccessStream, Windows.Storage.Streams, ContentType = WindowsRuntime]
$file = Await ([Windows.Storage.StorageFile]::GetFileFromPathAsync($env:ZRSJ_PDF_IN)) ([Windows.Storage.StorageFile])
$doc = Await ([Windows.Data.Pdf.PdfDocument]::LoadFromFileAsync($file)) ([Windows.Data.Pdf.PdfDocument])
if ($doc.PageCount -lt 1) { exit 1 }
$pdfPage = $doc.GetPage(0)
$stream = New-Object Windows.Storage.Streams.InMemoryRandomAccessStream
$opts = New-Object Windows.Data.Pdf.PdfPageRenderOptions
$opts.DestinationWidth = 1600
AwaitAction ($pdfPage.RenderToStreamAsync($stream, $opts))
$reader = New-Object Windows.Storage.Streams.DataReader($stream.GetInputStreamAt(0))
$size = [uint32]$stream.Size
Await ($reader.LoadAsync($size)) ([uint32]) | Out-Null
$bytes = New-Object byte[] $size
$reader.ReadBytes($bytes)
[System.IO.File]::WriteAllBytes($env:ZRSJ_PDF_OUT, $bytes)
$reader.Dispose(); $stream.Dispose()
Write-Output 'ok'
"#;

fn run_ps(script: &str, env: &[(&str, &str)]) -> String {
    let encoded = {
        let utf16: Vec<u8> = script.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        base64(&utf16)
    };
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-NonInteractive", "-WindowStyle", "Hidden", "-EncodedCommand", &encoded]);
    for (k, v) in env {
        cmd.env(k, v);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    match cmd.output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => String::new(),
    }
}

/// 归一化 OCR 文本：全角转半角、去掉汉字间空格、修正常见粘连
pub fn normalize_ocr(raw: &str) -> String {
    let mut stage: Vec<char> = Vec::with_capacity(raw.len());
    for c in raw.chars() {
        let m = match c {
            '０'..='９' => char::from_u32(c as u32 - 0xFF10 + 0x30).unwrap_or(c),
            'Ａ'..='Ｚ' => char::from_u32(c as u32 - 0xFF21 + 0x41).unwrap_or(c),
            'ａ'..='ｚ' => char::from_u32(c as u32 - 0xFF41 + 0x61).unwrap_or(c),
            '．' => '.',
            '，' => ',',
            '：' => ':',
            '；' => ';',
            '％' => '%',
            '￥' => '¥',
            '（' => '(',
            '）' => ')',
            _ => c,
        };
        stage.push(m);
    }
    let s: String = stage.into_iter().collect();
    let s = s.replace("阝艮", "限").replace("石马", "码").replace("发 票", "发票");

    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &c) in chars.iter().enumerate() {
        if c == ' ' || c == '\t' || c == '\u{3000}' {
            let prev = chars[..i].iter().rev().find(|x| !x.is_whitespace()).copied();
            let next = chars[i + 1..].iter().find(|x| !x.is_whitespace()).copied();
            let keep = matches!((prev, next), (Some(p), Some(n)) if p.is_ascii_alphabetic() && n.is_ascii_alphabetic());
            if !keep {
                continue;
            }
        }
        out.push(c);
    }
    out
}

/// 定位内置 Tesseract 目录（随安装包分发，任何 Windows 都可用）
pub fn tesseract_dir() -> Option<std::path::PathBuf> {
    let mut cands: Vec<std::path::PathBuf> = Vec::new();
    if let Ok(p) = std::env::var("ZRSJ_TESSERACT_DIR") {
        cands.push(std::path::PathBuf::from(p));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            // Tauri 原生安装包：<安装目录>\resources\tesseract
            cands.push(dir.join("resources").join("tesseract"));
            // 自定义 NSIS：<安装目录>\tesseract（旧布局）
            cands.push(dir.join("tesseract"));
            // 开发态：target/release → src-tauri/resources/tesseract
            cands.push(dir.join("../../resources/tesseract"));
        }
    }
    cands.push(data_dir().join("tesseract"));
    for c in cands {
        if c.join("tesseract.exe").exists() && c.join("tessdata").exists() {
            return Some(c);
        }
    }
    None
}

fn ocr_with_tesseract(dir: &std::path::Path, image: &str) -> Option<String> {
    let mut cmd = std::process::Command::new(dir.join("tesseract.exe"));
    cmd.args([
        image,
        "stdout",
        "--tessdata-dir",
        &dir.join("tessdata").to_string_lossy(),
        "-l",
        "chi_sim+eng",
        "--psm",
        "6",
    ]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout).to_string();
    if text.trim().is_empty() {
        None
    } else {
        Some(normalize_ocr(&text))
    }
}

/// 对图片做 OCR：内置引擎优先；没有内置引擎时退回系统 OCR
pub fn ocr_image(path: &str) -> String {
    if let Some(dir) = tesseract_dir() {
        if let Some(text) = ocr_with_tesseract(&dir, path) {
            return text;
        }
    }
    let out = run_ps(OCR_PS, &[("ZRSJ_OCR_IN", path)]);
    normalize_ocr(&out)
}

/// 渲染 PDF 首页为 PNG（输出路径由调用方给出），失败返回 false
pub fn pdf_first_page_png(pdf: &str, png_out: &str) -> bool {
    let out = run_ps(PDF2PNG_PS, &[("ZRSJ_PDF_IN", pdf), ("ZRSJ_PDF_OUT", png_out)]);
    out.trim().contains("ok") && std::path::Path::new(png_out).exists()
}


/// OFD 是 zip + XML：抽取 TextCode 文本节点（够用于票号/金额/名称识别）
pub fn ofd_text(path: &str) -> String {
    use std::io::Read;
    let Ok(file) = std::fs::File::open(path) else {
        return String::new();
    };
    let Ok(mut zip) = zip::ZipArchive::new(file) else {
        return String::new();
    };
    let mut out = String::new();
    for i in 0..zip.len() {
        let Ok(mut f) = zip.by_index(i) else { continue };
        if !f.name().to_lowercase().ends_with(".xml") {
            continue;
        }
        let mut buf = String::new();
        if f.read_to_string(&mut buf).is_err() {
            continue;
        }
        let mut rest = buf.as_str();
        while let Some(pos) = rest.find("TextCode") {
            let after = &rest[pos..];
            let Some(gt) = after.find('>') else { break };
            let content_start = &after[gt + 1..];
            let Some(end) = content_start.find("</") else { break };
            let text = &content_start[..end];
            if !text.trim().is_empty() {
                out.push_str(text);
                out.push('\n');
            }
            rest = &content_start[end..];
        }
    }
    out
}

fn ext_of(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase()
}

/// 导入本地发票附件：归档到 data/attachments/local/<yyyymm>/，解析字段后入夹
pub fn import_files(conn: &Connection, paths: &[String]) -> Vec<ImportResult> {
    let company: Option<String> = conn
        .query_row("SELECT name FROM company LIMIT 1", [], |r| r.get(0))
        .ok();
    let month = {
        let now = crate::db::now_iso();
        // now_iso 为秒级时间戳字符串，这里直接用当天日期
        let _ = now;
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // 粗略换算年月：用 chrono 未引入，直接按 1970 起算
        let days = secs / 86_400;
        let (y, m) = civil_from_days(days as i64);
        format!("{y:04}{m:02}")
    };

    let mut out = Vec::new();
    for p in paths {
        let path = std::path::Path::new(p);
        let name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment")
            .to_string();
        let mut r = ImportResult {
            file: name.clone(),
            ok: false,
            duplicate: false,
            invoice_no: None,
            amount: None,
            issued_on: None,
            seller: None,
            category: None,
            message: String::new(),
        };
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => {
                r.message = format!("读取失败：{e}");
                out.push(r);
                continue;
            }
        };
        let sha = sha256_file(&bytes);
        let ext = ext_of(p);
        if !matches!(ext.as_str(), "pdf" | "ofd" | "jpg" | "jpeg" | "png" | "bmp" | "webp") {
            r.message = format!("暂不支持 .{ext} 格式");
            out.push(r);
            continue;
        }

        // 解析（仅 PDF 可提取文本）
        // PDF 先抽文字（pdf-extract 在 CID/Identity-H 字体会 assert，用 catch_unwind 兜住）；
        // 抽不到（扫描件/图片）再走 Windows OCR
        let mut used_ocr = false;
        let mut text = if ext == "pdf" {
            let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pdf_extract::extract_text(path)
            }));
            match res {
                Ok(Ok(s)) => s,
                Ok(Err(_)) | Err(_) => String::new(),
            }
        } else if ext == "ofd" {
            ofd_text(p)
        } else {
            String::new()
        };
        let thin = text.chars().filter(|c| !c.is_whitespace()).count() < 30;
        if thin {
            if ext == "pdf" {
                let png = std::env::temp_dir().join(format!("zrsj-ocr-{}.png", &sha[..12]));
                let png_s = png.to_string_lossy().to_string();
                if pdf_first_page_png(p, &png_s) {
                    let ocr = ocr_image(&png_s);
                    let _ = std::fs::remove_file(&png);
                    if ocr.chars().filter(|c| !c.is_whitespace()).count() >= 10 {
                        text = ocr;
                        used_ocr = true;
                    }
                }
            } else if matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp") {
                let ocr = ocr_image(p);
                if !ocr.trim().is_empty() {
                    text = ocr;
                    used_ocr = true;
                }
            }
        }
        if !text.is_empty() {
            r.invoice_no = find_invoice_no(&text);
            r.amount = find_amount(&text);
            r.issued_on = find_date(&text);
            let (seller, buyer) = find_parties(&text, company.as_deref());
            r.seller = seller;
            r.category = Some(categorize(r.seller.as_deref(), &text));
            let _ = buyer;
        }
        if r.category.is_none() {
            r.category = Some(categorize(None, &name));
        }

        // 去重：sha256 或票号
        let dup_by_sha: i64 = conn
            .query_row("SELECT COUNT(*) FROM voucher WHERE sha256=?1", [&sha], |r| r.get(0))
            .unwrap_or(0);
        let dup_by_no: i64 = match &r.invoice_no {
            Some(no) => conn
                .query_row("SELECT COUNT(*) FROM voucher WHERE invoice_no=?1", [no], |r| r.get(0))
                .unwrap_or(0),
            None => 0,
        };
        if dup_by_sha > 0 || dup_by_no > 0 {
            r.duplicate = true;
            r.ok = true;
            r.message = "已存在，跳过".into();
            out.push(r);
            continue;
        }

        // 归档
        let dir = data_dir().join("attachments").join("local").join(&month);
        if let Err(e) = std::fs::create_dir_all(&dir) {
            r.message = format!("建目录失败：{e}");
            out.push(r);
            continue;
        }
        let stem = r
            .invoice_no
            .clone()
            .unwrap_or_else(|| sha.chars().take(12).collect());
        let file = dir.join(format!("{stem}.{ext}"));
        if let Err(e) = std::fs::write(&file, &bytes) {
            r.message = format!("写附件失败：{e}");
            out.push(r);
            continue;
        }

        let invoice_no = r
            .invoice_no
            .clone()
            .unwrap_or_else(|| format!("LOCAL-{}", &sha[..10]));
        let res = conn.execute(
            "INSERT INTO voucher(id,kind,invoice_code,invoice_no,status,book_status,reimburse_status,
               expense_category,amount,issued_on,seller,file_path,archive_dir,sha256,is_original,created_at,updated_at)
             VALUES(?1,'invoice',NULL,?2,'normal','unbooked','none',?3,?4,?5,?6,?7,?8,?9,1,?10,?10)",
            rusqlite::params![
                new_id(),
                invoice_no,
                r.category.clone(),
                r.amount.clone(),
                r.issued_on.clone(),
                r.seller.clone(),
                file.to_string_lossy(),
                dir.to_string_lossy(),
                sha,
                crate::db::now_iso(),
            ],
        );
        match res {
            Ok(_) => {
                r.ok = true;
                r.message = if ext == "ofd" && r.amount.is_none() {
                    "OFD 已归档（未识别到关键字段，可补录）".into()
                } else if used_ocr {
                    if r.amount.is_some() { "已 OCR 识别并归档".into() } else { "已 OCR（未识别到金额，可补录）".into() }
                } else if r.amount.is_some() {
                    "已归档并入夹".into()
                } else {
                    "已归档（未识别到金额，可补录）".into()
                };
            }
            Err(e) => r.message = format!("入库失败：{e}"),
        }
        out.push(r);
    }

    // 批次与审计：一次导入 = 一个批次
    let ok_count = out.iter().filter(|r| r.ok && !r.duplicate).count() as i64;
    if !out.is_empty() {
        let first = out
            .first()
            .map(|r| r.file.clone())
            .unwrap_or_else(|| "batch".into());
        let hash = format!("files:{}:{}", out.len(), crate::db::now_iso());
        crate::audit::log_batch(conn, "invoice_file", &first, &hash, ok_count);
        crate::audit::log_change(
            conn,
            "voucher",
            "",
            "insert",
            &format!("附件导入 {ok_count} 张（共 {} 个文件）", out.len()),
        );
    }
    out
}

/// 从 1970-01-01 起的天数换算 (year, month)
fn civil_from_days(z: i64) -> (i64, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as i64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m)
}

// ── 邮箱配置（持久化到 schema_meta，授权码本机加密保存）───────────

#[derive(Serialize, Deserialize, Clone)]
pub struct MailConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    #[serde(default)]
    pub pass_enc: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub auto: bool,
    #[serde(default)]
    pub last_fetch: String,
}

#[derive(Serialize)]
pub struct MailConfigView {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub folder: String,
    pub auto: bool,
    pub last_fetch: String,
    pub has_pass: bool,
}

fn read_cfg(conn: &Connection) -> Option<MailConfig> {
    let raw: Option<String> = conn
        .query_row("SELECT value FROM schema_meta WHERE key='mail_config'", [], |r| r.get(0))
        .ok();
    raw.and_then(|s| serde_json::from_str::<MailConfig>(&s).ok())
}

fn write_cfg(conn: &Connection, cfg: &MailConfig) -> Result<(), String> {
    let json = serde_json::to_string(cfg).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('mail_config',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [json],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn config_get(conn: &Connection) -> Option<MailConfigView> {
    read_cfg(conn).map(|c| MailConfigView {
        host: c.host,
        port: c.port,
        user: c.user,
        folder: if c.folder.is_empty() { "INBOX".into() } else { c.folder },
        auto: c.auto,
        last_fetch: c.last_fetch,
        has_pass: !c.pass_enc.is_empty(),
    })
}

pub fn config_save(
    conn: &Connection,
    host: String,
    port: u16,
    user: String,
    pass: Option<String>,
    folder: Option<String>,
    auto: bool,
) -> Result<(), String> {
    let mut cfg = read_cfg(conn).unwrap_or(MailConfig {
        host: String::new(),
        port: 993,
        user: String::new(),
        pass_enc: String::new(),
        folder: "INBOX".into(),
        auto: true,
        last_fetch: String::new(),
    });
    cfg.host = host;
    cfg.port = port;
    cfg.user = user;
    if let Some(p) = pass {
        if !p.is_empty() {
            cfg.pass_enc = crate::crypto::enc_secret(&p);
        }
    }
    cfg.folder = folder.unwrap_or_else(|| "INBOX".into());
    cfg.auto = auto;
    write_cfg(conn, &cfg)
}

pub fn config_clear(conn: &Connection) -> Result<(), String> {
    conn.execute("DELETE FROM schema_meta WHERE key='mail_config'", [])
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 用已保存配置收票；返回 (报告, 是否到期自动执行)
pub fn fetch_with_saved(
    conn: &Connection,
    force: bool,
) -> Result<(crate::mail::FetchReport, bool), String> {
    let cfg = read_cfg(conn).ok_or("尚未绑定邮箱")?;
    if cfg.user.is_empty() || cfg.pass_enc.is_empty() {
        return Err("尚未绑定邮箱".into());
    }
    if !force {
        if !cfg.auto {
            return Err("未开启自动收票".into());
        }
        // 10 分钟内不重复
        if let Ok(last) = cfg.last_fetch.parse::<u64>() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if now.saturating_sub(last) < 600 {
                return Err("刚刚收过票，稍后再试".into());
            }
        }
    }
    let pass = crate::crypto::dec_secret(&cfg.pass_enc).ok_or("授权码解密失败，请重新填写")?;
    let folder = if cfg.folder.is_empty() { "INBOX".to_string() } else { cfg.folder.clone() };
    let report = crate::mail::fetch_invoices(conn, &cfg.host, cfg.port, &cfg.user, &pass, &folder)?;

    let mut cfg2 = cfg.clone();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    cfg2.last_fetch = now.to_string();
    let _ = write_cfg(conn, &cfg2);
    Ok((report, true))
}

/// 调用系统文件选择框（PowerShell + WinForms，无需额外依赖）
pub fn pick_files() -> Vec<String> {
    let ps = r#"
Add-Type -AssemblyName System.Windows.Forms | Out-Null
$d = New-Object System.Windows.Forms.OpenFileDialog
$d.Title = '选择发票附件'
$d.Filter = '发票文件|*.pdf;*.ofd;*.jpg;*.jpeg;*.png;*.bmp;*.webp|所有文件|*.*'
$d.Multiselect = $true
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  $d.FileNames | ForEach-Object { Write-Output $_ }
}
"#;
    let encoded = {
        // UTF-16LE + base64，避免引号/中文转义问题
        let utf16: Vec<u8> = ps
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        base64(&utf16)
    };
    let mut cmd = std::process::Command::new("powershell");
    cmd.args(["-NoProfile", "-STA", "-WindowStyle", "Hidden", "-EncodedCommand", &encoded]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let out = cmd.output();
    match out {
        Ok(o) => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn base64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::new();
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        s.push(T[(n >> 18) as usize & 63] as char);
        s.push(T[(n >> 12) as usize & 63] as char);
        s.push(if chunk.len() > 1 {
            T[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        s.push(if chunk.len() > 2 {
            T[n as usize & 63] as char
        } else {
            '='
        });
    }
    s
}

/// 供命令层使用：拿锁后调用
pub fn with_conn<R>(
    state: &DbState,
    f: impl FnOnce(&Connection) -> Result<R, String>,
) -> Result<R, String> {
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    let conn = guard.as_ref().ok_or("db not initialized")?;
    f(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db() -> (std::path::PathBuf, Connection) {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-intake-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("t.db");
        let _ = std::fs::remove_file(&path);
        let conn = crate::db::open_and_migrate(&path).expect("open db");
        conn.execute(
            "INSERT INTO company(id,name,tax_no,taxpayer_type,filing_cycle,created_at,updated_at)
             VALUES('c1','自然拾光科技（广州）有限公司','91440101MA00000000','small','quarter','0','0')",
            [],
        )
        .expect("company");
        (dir, conn)
    }

    #[test]
    fn import_parses_and_archives() {
        let pdf = std::env::temp_dir().join("test_invoice.pdf");
        if !pdf.exists() {
            eprintln!("skip: sample pdf missing");
            return;
        }
        let (dir, conn) = temp_db();
        let out = import_files(&conn, &[pdf.to_string_lossy().to_string()]);
        assert_eq!(out.len(), 1);
        let r = &out[0];
        assert!(r.ok, "import failed: {}", r.message);
        assert_eq!(r.invoice_no.as_deref(), Some("24312000000012345678"));
        assert_eq!(r.amount.as_deref(), Some("128.50"));
        assert_eq!(r.issued_on.as_deref(), Some("2026-10-20"));
        assert_eq!(r.category.as_deref(), Some("餐饮"));

        let (no, fp, sha): (String, String, String) = conn
            .query_row("SELECT invoice_no,file_path,sha256 FROM voucher LIMIT 1", [], |x| {
                Ok((x.get(0)?, x.get(1)?, x.get(2)?))
            })
            .expect("voucher row");
        assert_eq!(no, "24312000000012345678");
        assert!(std::path::Path::new(&fp).exists(), "archived file missing");
        assert_eq!(sha.len(), 64);

        let again = import_files(&conn, &[pdf.to_string_lossy().to_string()]);
        assert!(again[0].duplicate, "should be duplicate");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn mail_config_roundtrip() {
        let (dir, conn) = temp_db();
        config_save(
            &conn,
            "imap.qq.com".into(),
            993,
            "a@qq.com".into(),
            Some("secret".into()),
            Some("INBOX".into()),
            true,
        )
        .expect("save");
        let v = config_get(&conn).expect("view");
        assert_eq!(v.user, "a@qq.com");
        assert!(v.has_pass && v.auto);
        let raw: String = conn
            .query_row("SELECT value FROM schema_meta WHERE key='mail_config'", [], |r| r.get(0))
            .unwrap();
        assert!(!raw.contains("\"secret\""));
        assert_eq!(crate::crypto::dec_secret(&crate::crypto::enc_secret("x")), Some("x".into()));
        config_clear(&conn).expect("clear");
        assert!(config_get(&conn).is_none());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn ocr_reads_image_and_scanned_pdf() {
        let tess = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("nsis/assets/tesseract");
        if !tess.join("tesseract.exe").exists() {
            eprintln!("skip: bundled tesseract missing");
            return;
        }
        unsafe { std::env::set_var("ZRSJ_TESSERACT_DIR", &tess) };

        let img = std::env::temp_dir().join("ocr_sample.png");
        if !img.exists() {
            eprintln!("skip: sample image missing");
            return;
        }
        let text = ocr_image(&img.to_string_lossy());
        assert!(
            text.contains("24312000000012345678"),
            "ocr text should contain invoice no, got: {text}"
        );

        let scan = std::env::temp_dir().join("scan_invoice.pdf");
        if !scan.exists() {
            eprintln!("skip: scanned pdf missing");
            return;
        }
        let (dir, conn) = temp_db();
        let out = import_files(&conn, &[scan.to_string_lossy().to_string()]);
        let r = &out[0];
        assert!(r.ok, "import failed: {}", r.message);
        assert_eq!(r.invoice_no.as_deref(), Some("24312000000012345678"), "msg={}", r.message);
        assert_eq!(r.amount.as_deref(), Some("128.50"), "msg={}", r.message);
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// 打开票据附件：只允许打开数据目录内、且确实存在的文件
pub fn open_attachment(conn: &Connection, voucher_id: &str) -> Result<String, String> {
    let path: Option<String> = conn
        .query_row("SELECT file_path FROM voucher WHERE id=?1", [voucher_id], |r| r.get(0))
        .ok()
        .flatten();
    let path = path.ok_or("该发票没有附件")?;
    let file = std::path::PathBuf::from(&path);
    if !file.exists() {
        return Err(format!("附件不存在：{path}"));
    }
    let root = data_dir().join("attachments");
    let root = root.canonicalize().unwrap_or(root);
    let target = file.canonicalize().unwrap_or_else(|_| file.clone());
    if !target.starts_with(&root) {
        return Err("附件不在数据目录内，已拒绝打开".into());
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        std::process::Command::new("explorer")
            .arg(target.as_os_str())
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("打开失败：{e}"))?;
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("xdg-open")
            .arg(target.as_os_str())
            .spawn()
            .map_err(|e| format!("打开失败：{e}"))?;
    }
    Ok(target.display().to_string())
}
/// 选目录（导出用）
pub fn pick_folder() -> Option<String> {
    let ps = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms | Out-Null
$d = New-Object System.Windows.Forms.FolderBrowserDialog
$d.Description = '选择导出目录'
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.SelectedPath }
"#;
    let out = run_ps(ps, &[]);
    let s = out.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}

/// 选一个 .db 文件（换机恢复用）
pub fn pick_db_file() -> Option<String> {
    let ps = r#"
$OutputEncoding = [Console]::OutputEncoding = [Text.Encoding]::UTF8
Add-Type -AssemblyName System.Windows.Forms | Out-Null
$d = New-Object System.Windows.Forms.OpenFileDialog
$d.Title = '选择账套数据库（zrsj.db）'
$d.Filter = '账套数据库|*.db|所有文件|*.*'
if ($d.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { Write-Output $d.FileName }
"#;
    let out = run_ps(ps, &[]);
    let s = out.trim().to_string();
    if s.is_empty() { None } else { Some(s) }
}
