use std::num::ParseIntError;

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing env var {0}: {1}")]
    Missing(String, std::env::VarError),
    #[error("invalid value for {0}: {1}")]
    Invalid(String, ParseIntError),
}

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub jwt_expiry_seconds: u64,
    pub refresh_token_expiry_days: u64,
    pub totp_issuer: String,
    pub totp_encryption_key: String,
    pub domain: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let get = |key: &str| -> Result<String, ConfigError> {
            std::env::var(key).map_err(|e| ConfigError::Missing(key.to_string(), e))
        };
        let get_u64 = |key: &str, default: u64| -> Result<u64, ConfigError> {
            match std::env::var(key) {
                Ok(v) => v.parse().map_err(|e| ConfigError::Invalid(key.to_string(), e)),
                Err(_) => Ok(default),
            }
        };

        Ok(Self {
            database_url: get("DATABASE_URL")?,
            redis_url: get("REDIS_URL")?,
            jwt_secret: get("JWT_SECRET")?,
            jwt_expiry_seconds: get_u64("JWT_EXPIRY_SECONDS", 900)?,
            refresh_token_expiry_days: get_u64("REFRESH_TOKEN_EXPIRY_DAYS", 7)?,
            totp_issuer: std::env::var("TOTP_ISSUER").unwrap_or_else(|_| "RuneChat".to_string()),
            totp_encryption_key: get("TOTP_ENCRYPTION_KEY")?,
            domain: std::env::var("DOMAIN").unwrap_or_else(|_| "chat.moonrune.cc".to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_errors_on_missing_required_var() {
        std::env::remove_var("DATABASE_URL");
        let result = Config::from_env();
        assert!(matches!(result, Err(ConfigError::Missing(ref k, _)) if k == "DATABASE_URL"));
    }

    #[test]
    fn config_uses_defaults_for_optional_vars() {
        std::env::set_var("DATABASE_URL", "postgres://test");
        std::env::set_var("REDIS_URL", "redis://test");
        std::env::set_var("JWT_SECRET", "secret");
        std::env::set_var("TOTP_ENCRYPTION_KEY", "key");
        std::env::remove_var("JWT_EXPIRY_SECONDS");
        std::env::remove_var("REFRESH_TOKEN_EXPIRY_DAYS");
        std::env::remove_var("TOTP_ISSUER");
        std::env::remove_var("DOMAIN");

        let config = Config::from_env().unwrap();
        assert_eq!(config.jwt_expiry_seconds, 900);
        assert_eq!(config.refresh_token_expiry_days, 7);
        assert_eq!(config.totp_issuer, "RuneChat");
        assert_eq!(config.domain, "chat.moonrune.cc");
    }
}
