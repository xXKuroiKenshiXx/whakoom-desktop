//! Portable backup envelope shared with the browser client.
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

const ITERATIONS: u32 = 600_000;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: String,
    iterations: u32,
    salt: String,
    nonce: String,
    ciphertext: String,
}
pub fn encrypt(plain: &[u8], password: &str) -> Result<Vec<u8>, String> {
    if password.chars().count() < 12 {
        return Err("Usá una contraseña de al menos 12 caracteres para el respaldo".into());
    }
    let mut salt = [0; 16];
    let mut nonce = [0; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let key = derive(password, &salt, ITERATIONS);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Clave inválida")?;
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plain)
        .map_err(|_| "No se pudo cifrar el respaldo")?;
    serde_json::to_vec(&Envelope {
        format: "whakoom-backup-v1".into(),
        iterations: ITERATIONS,
        salt: STANDARD.encode(salt),
        nonce: STANDARD.encode(nonce),
        ciphertext: STANDARD.encode(ciphertext),
    })
    .map_err(|e| e.to_string())
}
pub fn decrypt(bytes: &[u8], password: &str) -> Result<Zeroizing<Vec<u8>>, String> {
    if bytes.len() > 90 * 1024 * 1024 {
        return Err("Respaldo demasiado grande".into());
    }
    let envelope: Envelope =
        serde_json::from_slice(bytes).map_err(|_| "Respaldo cifrado inválido")?;
    if envelope.format != "whakoom-backup-v1" || envelope.iterations != ITERATIONS {
        return Err("Formato de respaldo no compatible".into());
    }
    let salt = STANDARD.decode(envelope.salt).map_err(|_| "Sal inválida")?;
    let nonce = STANDARD
        .decode(envelope.nonce)
        .map_err(|_| "Nonce inválido")?;
    if salt.len() != 16 || nonce.len() != 12 {
        return Err("Parámetros de cifrado inválidos".into());
    }
    let data = STANDARD
        .decode(envelope.ciphertext)
        .map_err(|_| "Datos inválidos")?;
    let key = derive(password, &salt, envelope.iterations);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|_| "Clave inválida")?;
    cipher
        .decrypt(Nonce::from_slice(&nonce), data.as_ref())
        .map(Zeroizing::new)
        .map_err(|_| "Contraseña incorrecta o respaldo dañado".into())
}
fn derive(password: &str, salt: &[u8], iterations: u32) -> Zeroizing<Vec<u8>> {
    let mut key = Zeroizing::new(vec![0; 32]);
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, iterations, &mut key);
    key
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn webcrypto_backups_can_be_restored_by_the_native_client() {
        let data = decrypt(
            include_bytes!("../web/tests/portable-fixture.json"),
            "portable-test-password",
        )
        .unwrap();
        let library: crate::storage::Library = serde_json::from_slice(&data).unwrap();
        assert_eq!(library.owner, "portable-test");
        library.validate().unwrap();
    }
    #[test]
    fn portable_backup_authenticates_and_randomises_every_export() {
        let password = "twelve words of a long secret";
        let first = encrypt(b"private notes", password).unwrap();
        let second = encrypt(b"private notes", password).unwrap();
        assert_ne!(first, second);
        assert_eq!(&**decrypt(&first, password).unwrap(), b"private notes");
        assert!(decrypt(&first, "wrong password").is_err());
        let mut envelope: Envelope = serde_json::from_slice(&first).unwrap();
        let mut cipher = STANDARD.decode(&envelope.ciphertext).unwrap();
        cipher[0] ^= 1;
        envelope.ciphertext = STANDARD.encode(cipher);
        assert!(decrypt(&serde_json::to_vec(&envelope).unwrap(), password).is_err());
        envelope.iterations = u32::MAX;
        assert!(decrypt(&serde_json::to_vec(&envelope).unwrap(), password).is_err());
    }
}
