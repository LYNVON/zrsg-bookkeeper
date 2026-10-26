// 数据库加密：SQLCipher + 本机密钥（DPAPI 保护）
// - 工作库始终加密；密钥存 data/key.bin，用 Windows DPAPI（CurrentUser）包裹
// - 明文旧库首次打开时自动迁移为加密库（迁移前先备份）
// - 导出给用户的是明文快照（便于换机），备份仍是加密库
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub fn key_file() -> PathBuf {
    crate::db::data_dir().join("key.bin")
}

pub fn is_plaintext(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let mut buf = [0u8; 16];
    f.read_exact(&mut buf).is_ok() && &buf == b"SQLite format 3\0"
}

#[cfg(windows)]
fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPT_INTEGER_BLOB};
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut out,
        );
        if ok == 0 {
            return Err("DPAPI 加密失败".into());
        }
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut core::ffi::c_void);
        Ok(bytes)
    }
}

#[cfg(windows)]
fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPT_INTEGER_BLOB};
    unsafe {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut out = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };
        let ok = CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            0,
            &mut out,
        );
        if ok == 0 {
            return Err("DPAPI 解密失败（可能换了 Windows 账户或机器）".into());
        }
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as *mut core::ffi::c_void);
        Ok(bytes)
    }
}

/// 读取或生成本机密钥（64 位 hex，供 PRAGMA key 使用）
pub fn local_key() -> Result<String, String> {
    let path = key_file();
    if path.exists() {
        let raw = std::fs::read(&path).map_err(|e| format!("读取密钥失败：{e}"))?;
        let plain = unprotect(&raw)?;
        if plain.len() != 32 {
            return Err("密钥文件损坏".into());
        }
        return Ok(hex(&plain));
    }
    let mut key = [0u8; 32];
    getrandom::getrandom(&mut key).map_err(|e| format!("生成密钥失败：{e}"))?;
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let wrapped = protect(&key)?;
    std::fs::write(&path, wrapped).map_err(|e| format!("写入密钥失败：{e}"))?;
    Ok(hex(&key))
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// 用原始密钥（x'hex'）设置 PRAGMA key —— 必须与 ATTACH 时的格式一致
fn apply_key(conn: &Connection, key_hex: &str) -> Result<(), String> {
    conn.execute_batch(&format!("PRAGMA key = \"x'{key_hex}'\";"))
        .map_err(|e| format!("设置密钥失败：{e}"))?;
    Ok(())
}

/// 打开工作库：加密打开 + 迁移明文 + 结构迁移
pub fn open_app_db(path: &Path) -> Result<Connection, String> {
    let key = local_key()?;

    if path.exists() && is_plaintext(path) {
        // 旧明文库：先备份再加密迁移
        let backup = crate::db::data_dir()
            .join("backups")
            .join(format!("pre-encrypt-{}.db", crate::db::now_iso()));
        if let Some(dir) = backup.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::copy(path, &backup);
        encrypt_existing(path, &key)?;
    }

    let conn = Connection::open(path).map_err(|e| e.to_string())?;
    apply_key(&conn, &key)?;
    // 触发一次读取，确认密钥正确
    conn.query_row("SELECT count(*) FROM sqlite_master", [], |r| r.get::<_, i64>(0))
        .map_err(|_| "数据库无法解密：密钥不匹配（key.bin 丢失或被替换？）".to_string())?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
        .map_err(|e| e.to_string())?;
    crate::db::migrate_schema(&conn).map_err(|e| e.to_string())?;
    Ok(conn)
}

/// 把明文 SQLite 就地加密（sqlcipher_export）
fn encrypt_existing(path: &Path, key: &str) -> Result<(), String> {
    let tmp = path.with_extension("enc-tmp");
    let _ = std::fs::remove_file(&tmp);
    {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(&format!(
            "ATTACH DATABASE '{}' AS enc KEY \"x'{}'\";
             SELECT sqlcipher_export('enc');
             DETACH DATABASE enc;",
            tmp.display().to_string().replace('\'', "''"),
            key
        ))
        .map_err(|e| format!("加密迁移失败：{e}"))?;
    }
    std::fs::remove_file(path).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// 把外部明文库导入为加密工作库（换机恢复）
pub fn import_plaintext(path: &Path, key: &str) -> Result<(), String> {
    if !is_plaintext(path) {
        return Err("不是明文 SQLite 数据库".into());
    }
    encrypt_existing(path, key)
}

/// 导出明文快照（供用户换机/存档）
pub fn export_plaintext(conn: &Connection, dest: &Path) -> Result<(), String> {
    let _ = std::fs::remove_file(dest);
    conn.execute_batch(&format!(
        "ATTACH DATABASE '{}' AS plain KEY '';
         SELECT sqlcipher_export('plain');
         DETACH DATABASE plain;",
        dest.display().to_string().replace('\'', "''")
    ))
    .map_err(|e| format!("导出明文库失败：{e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plaintext_detect_and_roundtrip() {
        let uniq = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("zrsj-enc-{}-{uniq}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let plain = dir.join("p.db");
        // 造一个明文库
        {
            let c = crate::db::open_and_migrate(&plain).expect("plain open");
            c.execute(
                "INSERT INTO company(id,name,tax_no,taxpayer_type,filing_cycle,created_at,updated_at)
                 VALUES('main','甲','T1','small','quarter','0','0')",
                [],
            )
            .unwrap();
        }
        assert!(is_plaintext(&plain), "应为明文库");
        // 用固定 key 加密迁移
        let key = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
        encrypt_existing(&plain, key).expect("encrypt");
        assert!(!is_plaintext(&plain), "加密后不应再是明文头");

        // 用同一 key 能读出数据
        let c = Connection::open(&plain).unwrap();
        apply_key(&c, key).expect("set key");
        let name: String = c
            .query_row("SELECT name FROM company WHERE id='main'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(name, "甲");

        // 导出明文后可被普通 sqlite 读取
        let out = dir.join("out.db");
        export_plaintext(&c, &out).expect("export");
        assert!(is_plaintext(&out));
        let pc = crate::db::open_and_migrate(&out).expect("plain read");
        let n2: String = pc
            .query_row("SELECT name FROM company WHERE id='main'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n2, "甲");

        drop(pc);
        drop(c);
        let _ = std::fs::remove_dir_all(dir);
    }
}
