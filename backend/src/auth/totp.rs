use crate::error::AppError;
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng as AesOsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use rand::RngCore;
use totp_rs::{Algorithm, TOTP};

pub fn generate_secret() -> Vec<u8> {
    let mut secret = vec![0u8; 20];
    rand::thread_rng().fill_bytes(&mut secret);
    secret
}

fn cipher(key_b64: &str) -> crate::error::Result<Aes256Gcm> {
    let key_bytes = B64
        .decode(key_b64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("invalid TOTP key encoding: {e}")))?;
    Aes256Gcm::new_from_slice(&key_bytes)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP key must be 32 bytes: {e}")))
}

pub fn encrypt_secret(secret: &[u8], key_b64: &str) -> crate::error::Result<String> {
    let cipher = cipher(key_b64)?;
    let nonce = Aes256Gcm::generate_nonce(&mut AesOsRng);
    let ciphertext = cipher
        .encrypt(&nonce, secret)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP encrypt: {e}")))?;
    let mut combined = nonce.to_vec();
    combined.extend_from_slice(&ciphertext);
    Ok(B64.encode(combined))
}

pub fn decrypt_secret(blob_b64: &str, key_b64: &str) -> crate::error::Result<Vec<u8>> {
    let combined = B64
        .decode(blob_b64)
        .map_err(|e| AppError::Internal(anyhow::anyhow!("decode TOTP blob: {e}")))?;
    if combined.len() < 12 {
        return Err(AppError::Internal(anyhow::anyhow!("TOTP blob too short")));
    }
    let nonce = Nonce::from_slice(&combined[..12]);
    let cipher = cipher(key_b64)?;
    cipher.decrypt(nonce, &combined[12..]).map_err(|_| {
        AppError::Internal(anyhow::anyhow!(
            "TOTP decrypt failed — wrong key or corrupt blob"
        ))
    })
}

fn make_totp(secret: &[u8], username: &str, issuer: &str) -> crate::error::Result<TOTP> {
    TOTP::new(
        Algorithm::SHA1,
        6,
        1,
        30,
        secret.to_vec(),
        Some(issuer.to_string()),
        username.to_string(),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("TOTP init: {e}")))
}

pub fn qr_url(secret: &[u8], username: &str, issuer: &str) -> crate::error::Result<String> {
    Ok(make_totp(secret, username, issuer)?.get_url())
}

pub fn verify_code(
    secret: &[u8],
    code: &str,
    username: &str,
    issuer: &str,
) -> crate::error::Result<bool> {
    Ok(make_totp(secret, username, issuer)?
        .check_current(code)
        .unwrap_or(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> String {
        B64.encode([0u8; 32])
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let secret = generate_secret();
        let key = test_key();
        let blob = encrypt_secret(&secret, &key).unwrap();
        let recovered = decrypt_secret(&blob, &key).unwrap();
        assert_eq!(secret, recovered);
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let secret = generate_secret();
        let key1 = B64.encode([0u8; 32]);
        let key2 = B64.encode([1u8; 32]);
        let blob = encrypt_secret(&secret, &key1).unwrap();
        assert!(decrypt_secret(&blob, &key2).is_err());
    }

    #[test]
    fn different_encryptions_produce_different_blobs() {
        let secret = generate_secret();
        let key = test_key();
        let blob1 = encrypt_secret(&secret, &key).unwrap();
        let blob2 = encrypt_secret(&secret, &key).unwrap();
        assert_ne!(blob1, blob2, "nonces must differ");
    }
}
