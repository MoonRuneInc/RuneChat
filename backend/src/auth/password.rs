use crate::error::AppError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash(password: &str) -> crate::error::Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(anyhow::anyhow!("hash failed: {e}")))
}

pub fn verify(password: &str, hash: &str) -> crate::error::Result<bool> {
    let parsed = PasswordHash::new(hash)
        .map_err(|_| AppError::Internal(anyhow::anyhow!("invalid hash format")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_roundtrip() {
        let h = hash("hunter42").unwrap();
        assert!(verify("hunter42", &h).unwrap());
        assert!(!verify("wrong", &h).unwrap());
    }

    #[test]
    fn different_passwords_produce_different_hashes() {
        let h1 = hash("password1").unwrap();
        let h2 = hash("password1").unwrap();
        assert_ne!(h1, h2, "salts must differ");
    }
}
