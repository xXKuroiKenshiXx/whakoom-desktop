use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct Session {
    pub cookie: String,
    pub user_agent: String,
    #[serde(default)]
    pub username: String,
}

pub fn data_dir() -> PathBuf {
    if let Some(path) = std::env::var_os("WHAKOOM_DESKTOP_DATA_DIR") {
        return PathBuf::from(path);
    }
    #[cfg(not(windows))]
    {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| {
                std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
            })
            .unwrap_or_else(std::env::temp_dir);
        base.join("whakoom-desktop")
    }
    #[cfg(windows)]
    {
        let base = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("."));
        // Reuse an existing installation's data without moving an active session.
        let legacy = base.join("WhakoomNative");
        if legacy.exists() {
            legacy
        } else {
            base.join("WhakoomDesktop")
        }
    }
}

/// Clear the login browser profile without touching library, backups or covers.
pub fn clear_browser_storage() -> Result<(), String> {
    let root = data_dir();
    let target = root.join("webview");
    if !target.exists() {
        return Ok(());
    }
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    let target = target.canonicalize().map_err(|e| e.to_string())?;
    if target.parent() != Some(root.as_path()) || target.file_name().is_none_or(|s| s != "webview")
    {
        return Err("La carpeta de cookies no pertenece a los datos de esta app".into());
    }
    fs::remove_dir_all(target).map_err(|_| "La sesión se desconectó, pero el navegador aún tiene archivos abiertos. Cerrá la app y volvé a borrar las cookies".into())
}

#[cfg(windows)]
pub fn crypt(data: &[u8], encrypt: bool) -> Result<Vec<u8>, String> {
    use windows_sys::Win32::{Foundation::LocalFree, Security::Cryptography::*};
    let input = CRYPT_INTEGER_BLOB {
        cbData: data
            .len()
            .try_into()
            .map_err(|_| "Sesión demasiado grande")?,
        pbData: data.as_ptr().cast_mut(),
    };
    let mut output = CRYPT_INTEGER_BLOB {
        cbData: 0,
        pbData: std::ptr::null_mut(),
    };
    // DPAPI owns the output allocation; copy it before releasing with LocalFree.
    unsafe {
        let ok = if encrypt {
            CryptProtectData(
                &input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        } else {
            CryptUnprotectData(
                &input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        };
        if ok == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let result = std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec();
        LocalFree(output.pbData.cast());
        Ok(result)
    }
}

pub fn save(session: &Session) -> Result<(), String> {
    #[cfg(windows)]
    {
        let bytes =
            zeroize::Zeroizing::new(serde_json::to_vec(session).map_err(|e| e.to_string())?);
        let encrypted = crypt(&bytes, true)?;
        crate::storage::atomic_write(&data_dir().join("session.dpapi"), &encrypted)
    }
    #[cfg(target_os = "linux")]
    {
        let bytes =
            zeroize::Zeroizing::new(serde_json::to_string(session).map_err(|e| e.to_string())?);
        linux_entry()?.set_password(&bytes).map_err(|_| "No se pudo guardar la sesión en el llavero del sistema. Desbloqueá Secret Service (GNOME Keyring/KWallet) e intentá de nuevo".into())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        let _ = session;
        Err("Persistencia de sesión disponible sólo en Windows".into())
    }
}

pub fn load() -> Option<Session> {
    #[cfg(windows)]
    {
        serde_json::from_slice(&zeroize::Zeroizing::new(
            crypt(&fs::read(data_dir().join("session.dpapi")).ok()?, false).ok()?,
        ))
        .ok()
    }
    #[cfg(target_os = "linux")]
    {
        let value = zeroize::Zeroizing::new(linux_entry().ok()?.get_password().ok()?);
        serde_json::from_str(&value).ok()
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    {
        None
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test]
    fn dpapi_roundtrip_and_corruption_detection() {
        let plain = b"session-cookie-test";
        let encrypted = crypt(plain, true).unwrap();
        assert_ne!(&encrypted[..], plain);
        assert_eq!(crypt(&encrypted, false).unwrap(), plain);
        assert!(crypt(b"corrupted session", false).is_err());
    }
}

pub fn clear() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    {
        match linux_entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err(
                "No se pudo borrar la sesión del llavero. Desbloquealo e intentá de nuevo".into(),
            ),
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        match fs::remove_file(data_dir().join("session.dpapi")) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

#[cfg(target_os = "linux")]
fn linux_entry() -> Result<keyring::Entry, String> {
    let account = format!(
        "session-{}",
        crate::storage::key(&data_dir().to_string_lossy())
    );
    keyring::Entry::new("com.whakoom.desktop", &account)
        .map_err(|_| "Llavero del sistema no disponible".into())
}
