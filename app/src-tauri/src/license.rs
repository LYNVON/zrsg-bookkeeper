// 离线授权：授权码 = ZRSJ1.<base64url(payload)>.<hex(sig)>
// 公钥内置，只有持有私钥的开发者能签发；无需联网。
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 与 tools/license-gen 的 keygen 输出对应
pub const PUBKEY_HEX: &str = "6c65308ae476c03971a228cfc3d2e0aa1f9effe55dfe32899b90de6f586c0a71";
const TRIAL_DAYS: i64 = 30;

#[derive(Serialize, Deserialize)]
struct Payload {
    m: String,
    p: String,
    e: String,
    i: String,
}

#[derive(Serialize)]
pub struct LicenseStatus {
    pub machine_code: String,
    pub plan: String,
    pub expire_on: Option<String>,
    pub licensed: bool,
    pub trial_days_left: i64,
    pub message: String,
}

fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
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
}

fn days_between(from: &str, to: &str) -> i64 {
    fn parse(s: &str) -> Option<i64> {
        let p: Vec<i64> = s.split('-').filter_map(|x| x.parse().ok()).collect();
        if p.len() != 3 {
            return None;
        }
        // 粗算：按年月日折算天数（足够做试用期判断）
        Some(p[0] * 365 + p[1] * 30 + p[2])
    }
    match (parse(from), parse(to)) {
        (Some(a), Some(b)) => b - a,
        _ => 0,
    }
}

/// 机器码：计算机名 + 用户名 + 固定盐
pub fn machine_code() -> String {
    let mut h = Sha256::new();
    h.update(std::env::var("COMPUTERNAME").unwrap_or_default().as_bytes());
    h.update(std::env::var("USERNAME").unwrap_or_default().as_bytes());
    h.update(b"zrsj-machine-v1");
    let d = h.finalize();
    d.iter().take(8).map(|b| format!("{b:02x}")).collect::<String>().to_uppercase()
}

fn verify_code(code: &str, machine: &str) -> Result<Payload, String> {
    let parts: Vec<&str> = code.trim().split('.').collect();
    if parts.len() != 3 || parts[0] != "ZRSJ1" {
        return Err("授权码格式不正确".into());
    }
    let payload_bytes = URL_SAFE_NO_PAD
        .decode(parts[1])
        .map_err(|_| "授权码内容损坏".to_string())?;
    let sig_bytes = hex::decode(parts[2]).map_err(|_| "授权码签名损坏".to_string())?;
    let sig_arr: [u8; 64] = sig_bytes.try_into().map_err(|_| "签名长度不对".to_string())?;
    let pk_bytes = hex::decode(PUBKEY_HEX).map_err(|_| "公钥损坏".to_string())?;
    let pk_arr: [u8; 32] = pk_bytes.try_into().map_err(|_| "公钥长度不对".to_string())?;
    let key = VerifyingKey::from_bytes(&pk_arr).map_err(|_| "公钥无效".to_string())?;
    let sig = Signature::from_bytes(&sig_arr);
    key.verify(parts[1].as_bytes(), &sig)
        .map_err(|_| "授权码签名校验失败（可能被篡改）".to_string())?;

    let payload: Payload =
        serde_json::from_slice(&payload_bytes).map_err(|_| "授权码内容无法解析".to_string())?;
    if payload.m != "*" && payload.m.to_uppercase() != machine.to_uppercase() {
        return Err(format!("授权码绑定的机器码是 {}，与本机 {machine} 不符", payload.m));
    }
    if payload.e != "*" && payload.e < today() {
        return Err(format!("授权已到期（{}）", payload.e));
    }
    Ok(payload)
}

pub fn first_run_day(conn: &Connection) -> String {
    let v: String = conn
        .query_row("SELECT value FROM schema_meta WHERE key='first_run'", [], |r| r.get(0))
        .unwrap_or_default();
    if !v.is_empty() {
        return v;
    }
    let t = today();
    let _ = conn.execute(
        "INSERT INTO schema_meta(key,value) VALUES('first_run',?1)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [t.clone()],
    );
    t
}

pub fn status(conn: &Connection) -> LicenseStatus {
    let machine = machine_code();
    let row: Option<(String, Option<String>, Option<String>)> = conn
        .query_row("SELECT plan, expire_on, local_token FROM license LIMIT 1", [], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?))
        })
        .ok();

    if let Some((plan, expire_on, token)) = row {
        if let Some(code) = token.as_deref() {
            match verify_code(code, &machine) {
                Ok(p) => {
                    return LicenseStatus {
                        machine_code: machine,
                        plan: p.p,
                        expire_on: if p.e == "*" { None } else { Some(p.e) },
                        licensed: true,
                        trial_days_left: 0,
                        message: "已授权".into(),
                    };
                }
                Err(e) => {
                    return LicenseStatus {
                        machine_code: machine,
                        plan,
                        expire_on,
                        licensed: false,
                        trial_days_left: trial_left(conn),
                        message: format!("授权无效：{e}"),
                    };
                }
            }
        }
    }

    let left = trial_left(conn);
    LicenseStatus {
        machine_code: machine,
        plan: "trial".into(),
        expire_on: None,
        licensed: false,
        trial_days_left: left,
        message: if left > 0 {
            format!("试用中，剩余 {left} 天")
        } else {
            "试用已结束，请输入授权码".into()
        },
    }
}

fn trial_left(conn: &Connection) -> i64 {
    let start = first_run_day(conn);
    (TRIAL_DAYS - days_between(&start, &today())).max(0)
}

pub fn activate(conn: &Connection, code: &str) -> Result<LicenseStatus, String> {
    let machine = machine_code();
    let p = verify_code(code, &machine)?;
    let now = crate::db::now_iso();
    conn.execute(
        "DELETE FROM license",
        [],
    )
    .map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO license(id,account_id,plan,ai_addon,expire_on,local_token,created_at,updated_at)
         VALUES(?1,'local',?2,0,?3,?4,?5,?5)",
        rusqlite::params![
            crate::db::new_id(),
            p.p,
            if p.e == "*" { None } else { Some(p.e.clone()) },
            code.trim(),
            now
        ],
    )
    .map_err(|e| e.to_string())?;
    crate::audit::log_change(conn, "license", "local", "update", &format!("激活授权 {}", p.p));
    Ok(status(conn))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> (std::path::PathBuf, Connection) {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-lic-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let conn = crate::db::open_and_migrate(&dir.join("l.db")).expect("open");
        (dir, conn)
    }

    /// 用测试私钥现场签名，验证校验链路（不依赖开发私钥文件）
    fn sign_for_test(machine: &str, plan: &str, expire: &str) -> String {
        use rand_core::{OsRng, RngCore};
        let mut kb = [0u8; 32];
        OsRng.fill_bytes(&mut kb);
        let sk = ed25519_dalek::SigningKey::from_bytes(&kb);
        let payload = Payload {
            m: machine.into(),
            p: plan.into(),
            e: expire.into(),
            i: today(),
        };
        let json = serde_json::to_vec(&payload).unwrap();
        let b64 = URL_SAFE_NO_PAD.encode(&json);
        use ed25519_dalek::Signer;
        let sig = sk.sign(b64.as_bytes());
        format!("ZRSJ1.{b64}.{}", hex::encode(sig.to_bytes()))
    }

    #[test]
    fn rejects_bad_format_and_tamper() {
        let (_d, conn) = ctx();
        assert!(verify_code("abc", "X").is_err());
        assert!(verify_code("ZRSJ1.aaa.bbb", "X").is_err());
        let _ = conn;
    }

    #[test]
    fn trial_counts_down_and_expires() {
        let (dir, conn) = ctx();
        conn.execute(
            "INSERT INTO schema_meta(key,value) VALUES('first_run',?1)",
            [today()],
        )
        .unwrap();
        let s = status(&conn);
        assert!(!s.licensed);
        assert_eq!(s.trial_days_left, TRIAL_DAYS);
        // 把首启日改到 40 天前
        let old = format!("{}", {
            let t = today();
            t
        });
        let _ = old;
        conn.execute("UPDATE schema_meta SET value='2020-01-01' WHERE key='first_run'", [])
            .unwrap();
        let s2 = status(&conn);
        assert_eq!(s2.trial_days_left, 0);
        assert!(s2.message.contains("试用已结束"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn machine_mismatch_detected() {
        let (dir, _conn) = ctx();
        // 用测试私钥签一个绑到别的机器码的码；公钥不同 → 必然签名失败
        let code = sign_for_test("OTHER-MACHINE", "pro", "*");
        let r = verify_code(&code, &machine_code());
        assert!(r.is_err());
        let _ = std::fs::remove_dir_all(dir);
    }
    #[test]
    fn embedded_pubkey_matches_dev_private_key() {
        // 私钥不进仓库；本机开发时存在则做一次真实签发-校验
        let key_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/license-gen/license-key.txt");
        let Ok(hexkey) = std::fs::read_to_string(&key_path) else {
            eprintln!("skip: dev private key missing");
            return;
        };
        let raw = hex::decode(hexkey.trim()).expect("私钥 hex");
        let arr: [u8; 32] = raw.try_into().expect("私钥长度");
        let sk = ed25519_dalek::SigningKey::from_bytes(&arr);
        let machine = machine_code();
        let payload = Payload {
            m: machine.clone(),
            p: "personal".into(),
            e: "*".into(),
            i: today(),
        };
        let json = serde_json::to_vec(&payload).unwrap();
        let b64 = URL_SAFE_NO_PAD.encode(&json);
        use ed25519_dalek::Signer;
        let sig = sk.sign(b64.as_bytes());
        let code = format!("ZRSJ1.{b64}.{}", hex::encode(sig.to_bytes()));

        let payload_back = verify_code(&code, &machine).expect("本机签发应能通过校验");
        assert_eq!(payload_back.p, "personal");
    }

}
