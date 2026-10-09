//! Private files are sealed before atomic replacement; exports are explicit.
use std::{fs, path::Path};
use zeroize::Zeroizing;

const MAGIC: &[u8] = b"WHAKOOM-VAULT-1\0";

pub fn write(path: &Path, plain: &[u8]) -> Result<(), String> {
    if path.exists() {
        let existing = Zeroizing::new(fs::read(path).map_err(|e| e.to_string())?);
        if let Some(cipher) = existing.strip_prefix(MAGIC) {
            // A locked keyring or broken file must not be replaced by defaults.
            let _ = Zeroizing::new(open_scoped(path, cipher)?);
        } else {
            let _: serde::de::IgnoredAny = serde_json::from_slice(&existing)
                .map_err(|_| "El archivo anterior está dañado; se conservó para recuperación")?;
        }
    }
    let mut scoped = Zeroizing::new(scope(path).to_vec());
    scoped.extend_from_slice(plain);
    let encrypted = seal(&scoped)?;
    crate::storage::atomic_write(path, &encrypted)
}

pub fn migrate_private_files() -> Result<(), String> {
    let root = crate::session::data_dir();
    let mut paths = vec![root.join("settings.json")];
    for directory in ["libraries", "pages"] {
        let directory = root.join(directory);
        if !directory.exists() {
            continue;
        }
        if fs::symlink_metadata(&directory)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("La carpeta de datos privados es un enlace; no se migró".into());
        }
        for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            if entry.file_type().map_err(|e| e.to_string())?.is_file()
                && entry.path().extension().is_some_and(|e| e == "json")
            {
                paths.push(entry.path());
            }
        }
    }
    for path in paths {
        if !path.exists() {
            continue;
        }
        if fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            continue;
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            continue;
        }
        let bytes = read(&path)?;
        // Leave broken legacy files intact for manual recovery.
        let _: serde::de::IgnoredAny = serde_json::from_slice(&bytes)
            .map_err(|_| "Hay un archivo privado inválido; se conservó para recuperación")?;
        migrate(&path, &bytes)?;
    }
    Ok(())
}

pub fn read(path: &Path) -> Result<Zeroizing<Vec<u8>>, String> {
    let bytes = fs::read(path).map_err(|e| e.to_string())?;
    if let Some(cipher) = bytes.strip_prefix(MAGIC) {
        return open_scoped(path, cipher).map(Zeroizing::new);
    }
    // Legacy JSON is migrated only after the caller validates it.
    Ok(Zeroizing::new(bytes))
}

fn scope(path: &Path) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    // Bind to the logical file while allowing the data directory to be moved.
    let parent = path
        .parent()
        .and_then(Path::file_name)
        .unwrap_or_default()
        .to_string_lossy();
    let namespace = if matches!(parent.as_ref(), "libraries" | "pages") {
        parent.as_ref()
    } else {
        "root"
    };
    let label = format!(
        "{}/{}",
        namespace,
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    Sha256::digest(label.as_bytes()).into()
}
fn open_scoped(path: &Path, cipher: &[u8]) -> Result<Vec<u8>, String> {
    let plain = Zeroizing::new(open(cipher)?);
    let expected = scope(path);
    if !plain.starts_with(&expected) {
        return Err("El archivo cifrado pertenece a otra sección".into());
    }
    Ok(plain[expected.len()..].to_vec())
}

pub fn migrate(path: &Path, plain: &[u8]) -> Result<(), String> {
    let bytes = Zeroizing::new(fs::read(path).map_err(|e| e.to_string())?);
    if !bytes.starts_with(MAGIC) {
        write(path, plain)?;
    }
    Ok(())
}

fn seal(plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut result = MAGIC.to_vec();
    #[cfg(windows)]
    result.extend(crate::session::crypt(plain, true)?);
    #[cfg(target_os = "linux")]
    {
        use aes_gcm::{
            Aes256Gcm,
            aead::{Aead, AeadCore, KeyInit, OsRng},
        };
        let key = linux_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Clave inválida")?;
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        result.extend_from_slice(&nonce);
        result.extend(
            cipher
                .encrypt(&nonce, plain)
                .map_err(|_| "No se pudieron cifrar los datos")?,
        );
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    return Err("Cifrado del sistema no disponible".into());
    Ok(result)
}

fn open(cipher: &[u8]) -> Result<Vec<u8>, String> {
    #[cfg(windows)]
    return crate::session::crypt(cipher, false).map_err(|_| {
        "No se pudieron descifrar los datos: cuenta de Windows distinta o archivo dañado".into()
    });
    #[cfg(target_os = "linux")]
    {
        use aes_gcm::{
            Aes256Gcm, Nonce,
            aead::{Aead, KeyInit},
        };
        let key = linux_key()?;
        let cipher_impl = Aes256Gcm::new_from_slice(&key).map_err(|_| "Clave inválida")?;
        if cipher.len() < 28 {
            return Err("Archivo cifrado incompleto".into());
        }
        cipher_impl
            .decrypt(Nonce::from_slice(&cipher[..12]), &cipher[12..])
            .map_err(|_| "Clave distinta o datos cifrados dañados".into())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    Err("Cifrado del sistema no disponible".into())
}

#[cfg(target_os = "linux")]
fn linux_key() -> Result<Zeroizing<Vec<u8>>, String> {
    use aes_gcm::aead::{OsRng, rand_core::RngCore};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use std::sync::{Mutex, OnceLock};
    // Serialise first creation; never replace a key when the keyring is locked.
    static KEY: OnceLock<Mutex<Option<Zeroizing<Vec<u8>>>>> = OnceLock::new();
    let mut cached = KEY
        .get_or_init(|| Mutex::new(None))
        .lock()
        .map_err(|_| "Llavero ocupado")?;
    if let Some(key) = cached.as_ref() {
        return Ok(key.clone());
    }
    let entry = keyring::Entry::new("com.whakoom.desktop", "private-files-v1")
        .map_err(|_| "Llavero no disponible")?;
    let key = match entry.get_password() {
        Ok(encoded) => Zeroizing::new(
            STANDARD
                .decode(Zeroizing::new(encoded).as_bytes())
                .map_err(|_| "Clave del llavero inválida")?,
        ),
        Err(keyring::Error::NoEntry) => {
            let mut key = Zeroizing::new(vec![0; 32]);
            OsRng.fill_bytes(&mut key);
            entry
                .set_password(&Zeroizing::new(STANDARD.encode(&*key)))
                .map_err(|_| "Desbloqueá Secret Service para guardar datos cifrados")?;
            key
        }
        Err(_) => return Err("Desbloqueá Secret Service para acceder a tus datos cifrados".into()),
    };
    if key.len() != 32 {
        return Err("Clave del llavero inválida".into());
    }
    *cached = Some(key.clone());
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[cfg_attr(
        target_os = "linux",
        ignore = "requires an unlocked, isolated Secret Service"
    )]
    fn private_file_is_not_plaintext_and_tampering_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("library.json");
        let plain = br#"{"notes":"private"}"#;
        write(&path, plain).unwrap();
        let mut bytes = fs::read(&path).unwrap();
        assert!(!bytes.windows(7).any(|w| w == b"private"));
        assert_eq!(&**read(&path).unwrap(), plain);
        let other = dir.path().join("different.json");
        fs::write(&other, &bytes).unwrap();
        assert!(read(&other).is_err());
        *bytes.last_mut().unwrap() ^= 1;
        fs::write(&path, bytes).unwrap();
        assert!(read(&path).is_err());
    }
}
