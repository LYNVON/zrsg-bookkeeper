// 离线授权码生成器（仅开发者在本机使用，私钥不随软件分发）
// 用法：
//   license-gen keygen                     生成密钥对，私钥写入 license-key.txt，打印公钥
//   license-gen sign <机器码> <plan> [到期YYYY-MM-DD]
//
// 授权码格式：ZRSJ1.<base64url(payload)>.<hex(sig)>
// payload = {"m":"机器码或*","p":"trial|personal|pro","e":"到期或*","i":"签发日"}
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::io::Write;

#[derive(Serialize, Deserialize)]
struct Payload {
    m: String, // 绑定机器码；"*" 表示通用
    p: String, // plan
    e: String, // 到期日 YYYY-MM-DD 或 "*"
    i: String, // 签发日
}

const KEY_FILE: &str = "license-key.txt";

fn keygen() -> Result<(), String> {
    let mut kb = [0u8; 32];
    OsRng.fill_bytes(&mut kb);
    let signing = SigningKey::from_bytes(&kb);
    let verify: VerifyingKey = signing.verifying_key();
    let priv_hex = hex::encode(signing.to_bytes());
    let pub_hex = hex::encode(verify.to_bytes());
    let mut f = std::fs::File::create(KEY_FILE).map_err(|e| e.to_string())?;
    f.write_all(priv_hex.as_bytes()).map_err(|e| e.to_string())?;
    println!("私钥已写入 {KEY_FILE}（务必保密、勿提交仓库）");
    println!("公钥（贴进 license.rs 的 PUBKEY_HEX）：");
    println!("{pub_hex}");
    Ok(())
}

fn load_key() -> Result<SigningKey, String> {
    let s = std::fs::read_to_string(KEY_FILE).map_err(|e| format!("读不到 {KEY_FILE}: {e}"))?;
    let bytes = hex::decode(s.trim()).map_err(|e| e.to_string())?;
    let arr: [u8; 32] = bytes.try_into().map_err(|_| "私钥长度不对".to_string())?;
    Ok(SigningKey::from_bytes(&arr))
}

fn sign(machine: &str, plan: &str, expire: &str) -> Result<String, String> {
    let key = load_key()?;
    let today = {
        // 用系统时间换算日期，避免引入 chrono
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0) as i64;
        let days = secs / 86_400;
        let z = days + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        let y = if m <= 2 { y + 1 } else { y };
        format!("{y:04}-{m:02}-{d:02}")
    };
    let payload = Payload {
        m: if machine.trim().is_empty() { "*".into() } else { machine.trim().to_uppercase() },
        p: plan.to_string(),
        e: if expire.trim().is_empty() { "*".into() } else { expire.trim().to_string() },
        i: today,
    };
    let json = serde_json::to_vec(&payload).map_err(|e| e.to_string())?;
    let b64 = URL_SAFE_NO_PAD.encode(&json);
    let sig: Signature = key.sign(b64.as_bytes());
    Ok(format!("ZRSJ1.{b64}.{}", hex::encode(sig.to_bytes())))
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let r = match args.first().map(|s| s.as_str()) {
        Some("keygen") => keygen(),
        Some("sign") => {
            if args.len() < 3 {
                Err("用法：license-gen sign <机器码|*> <trial|personal|pro> [到期YYYY-MM-DD]".into())
            } else {
                let expire = args.get(3).cloned().unwrap_or_default();
                sign(&args[1], &args[2], &expire).map(|code| {
                    println!("{code}");
                })
            }
        }
        _ => Err("用法：license-gen keygen | sign <机器码> <plan> [到期]".into()),
    };
    if let Err(e) = r {
        eprintln!("错误：{e}");
        std::process::exit(1);
    }
}
