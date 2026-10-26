// 加密备份（PRD §5.9 数据安全）：AES-256-GCM，口令 → SHA256 派生密钥
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Key, Nonce};
use sha2::{Digest, Sha256};

fn derive_key(password: &str) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(password.as_bytes());
    h.update(b"zrsj-backup-v1");
    let out = h.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&out);
    key
}

/// 加密备份：data/zrsj.db → data/backups/zrsj-enc-<ts>.zrsjbackup
pub fn backup_encrypted(db_path: &str, password: &str) -> Result<String, String> {
    let plain = std::fs::read(db_path).map_err(|e| format!("读库失败：{e}"))?;
    let key_bytes = derive_key(password);
    let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ct = cipher
        .encrypt(&nonce, plain.as_ref())
        .map_err(|_| "加密失败".to_string())?;
    let dir = crate::db::data_dir().join("backups");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let ts = crate::db::now_iso();
    let out = dir.join(format!("zrsj-enc-{ts}.zrsjbackup"));
    // 格式：magic(4) + nonce(12) + ciphertext
    let mut blob = b"ZRSJ".to_vec();
    blob.extend_from_slice(nonce.as_slice());
    blob.extend_from_slice(&ct);
    std::fs::write(&out, blob).map_err(|e| e.to_string())?;
    Ok(out.display().to_string())
}

/// 解密校验（不落库，仅验证口令与完整性）
pub fn verify_encrypted(path: &str, password: &str) -> Result<bool, String> {
    let blob = std::fs::read(path).map_err(|e| e.to_string())?;
    if blob.len() < 16 || &blob[..4] != b"ZRSJ" {
        return Err("不是有效的 zrsjbackup 文件".into());
    }
    let key_bytes = derive_key(password);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(&blob[4..16]);
    cipher
        .decrypt(nonce, &blob[16..])
        .map(|p| p.len() > 16)
        .map_err(|_| "口令错误或文件损坏".to_string())
}

// ── 本机密钥（保存邮箱授权码等敏感配置）────────────────────
// 由本机环境派生的固定盐生成，属「本机加密存储」：避免明文落库，
// 不等同于针对离线攻击者的强保护。

fn machine_key() -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(std::env::var("COMPUTERNAME").unwrap_or_default().as_bytes());
    h.update(std::env::var("USERNAME").unwrap_or_default().as_bytes());
    h.update(b"zrsj-local-secret-v1");
    let out = h.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&out);
    key
}

fn to_hex(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 2 <= bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        out.push((hi * 16 + lo) as u8);
        i += 2;
    }
    Some(out)
}

/// 加密短文本（IMAP 授权码）：hex(nonce || ciphertext)
pub fn enc_secret(plain: &str) -> String {
    let key = machine_key();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    match cipher.encrypt(&nonce, plain.as_bytes()) {
        Ok(ct) => {
            let mut blob = nonce.to_vec();
            blob.extend_from_slice(&ct);
            to_hex(&blob)
        }
        Err(_) => String::new(),
    }
}

pub fn dec_secret(enc: &str) -> Option<String> {
    let blob = from_hex(enc)?;
    if blob.len() < 16 {
        return None;
    }
    let key = machine_key();
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let nonce = Nonce::from_slice(&blob[..12]);
    let pt = cipher.decrypt(nonce, &blob[12..]).ok()?;
    String::from_utf8(pt).ok()
}

/// 解密备份包（magic ZRSJ + nonce + ciphertext）
pub fn decrypt_blob(blob: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if blob.len() < 20 || &blob[..4] != b"ZRSJ" {
        return Err("不是有效的 zrsjbackup 文件".into());
    }
    let key_bytes = derive_key(password);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key_bytes));
    let nonce = Nonce::from_slice(&blob[4..16]);
    cipher
        .decrypt(nonce, &blob[16..])
        .map_err(|_| "口令错误或文件损坏".to_string())
}
