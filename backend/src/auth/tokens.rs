use crate::{config::Config, error::AppError};
use hmac::{Hmac, Mac};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use time::OffsetDateTime;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub account_status: String,
    pub exp: usize,
}

pub fn encode_jwt(
    user_id: Uuid,
    username: &str,
    account_status: &str,
    config: &Config,
) -> crate::error::Result<String> {
    let exp =
        (OffsetDateTime::now_utc().unix_timestamp() as u64 + config.jwt_expiry_seconds) as usize;
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        account_status: account_status.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(anyhow::anyhow!("jwt encode: {e}")))
}

pub fn decode_jwt(token: &str, config: &Config) -> crate::error::Result<Claims> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::Unauthorized)
}

pub fn generate_refresh_token() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect()
}

pub fn hash_refresh_token(token: &str, secret: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts any key size");
    mac.update(token.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use base64::Engine;
    fn test_config() -> Config {
        Config {
            database_url: String::new(),
            redis_url: String::new(),
            jwt_secret: "test-secret-32-bytes-min-length!!".to_string(),
            jwt_expiry_seconds: 900,
            refresh_token_expiry_days: 7,
            totp_issuer: "Cauldron".to_string(),
            totp_encryption_key: base64::engine::general_purpose::STANDARD.encode([0u8; 32]),
            domain: "localhost".to_string(),
            smtp: None,
        }
    }

    #[test]
    fn jwt_encode_decode_roundtrip() {
        let config = test_config();
        let id = Uuid::new_v4();
        let token = encode_jwt(id, "alice", "active", &config).unwrap();
        let claims = decode_jwt(&token, &config).unwrap();
        assert_eq!(claims.sub, id.to_string());
        assert_eq!(claims.username, "alice");
        assert_eq!(claims.account_status, "active");
    }

    #[test]
    fn decode_rejects_wrong_secret() {
        let mut config = test_config();
        config.jwt_secret = "wrong-secret".to_string();
        let good = {
            let mut c = test_config();
            c.jwt_secret = "correct-secret-32-bytes-min-len!!".to_string();
            encode_jwt(Uuid::new_v4(), "bob", "active", &c).unwrap()
        };
        assert!(decode_jwt(&good, &config).is_err());
    }

    #[test]
    fn refresh_token_hash_is_deterministic() {
        let h1 = hash_refresh_token("token123", "secret");
        let h2 = hash_refresh_token("token123", "secret");
        assert_eq!(h1, h2);
    }

    #[test]
    fn different_tokens_produce_different_hashes() {
        let h1 = hash_refresh_token("tokenA", "secret");
        let h2 = hash_refresh_token("tokenB", "secret");
        assert_ne!(h1, h2);
    }
}
