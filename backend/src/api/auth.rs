use axum::{
    extract::{ConnectInfo, State},
    http::{header, HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use std::net::SocketAddr;
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;
use crate::{
    auth::{middleware::AuthUser, password, tokens, totp},
    error::AppError,
    pwned::{self, PwnedCheckResult},
    state::AppState,
};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
        .route("/totp/enroll", post(totp_enroll))
        .route("/totp/verify-enrollment", post(totp_verify_enrollment))
        .route("/unlock/totp", post(unlock_totp))
        .route("/unlock/email-otp/send", post(unlock_email_otp_send))
        .route("/unlock/email-otp/verify", post(unlock_email_otp_verify))
        .layer(tower_http::set_header::SetResponseHeaderLayer::if_not_present(
            header::CACHE_CONTROL,
            header::HeaderValue::from_static("no-store"),
        ))
}

// --- Shared types ---

#[derive(Serialize)]
struct UserInfo {
    id: Uuid,
    username: String,
    account_status: String,
}

#[derive(Serialize)]
struct AuthResponse {
    access_token: String,
    user: UserInfo,
}

fn validate_username(username: &str) -> crate::error::Result<()> {
    if username.len() < 2 || username.len() > 32 {
        return Err(AppError::BadRequest("username must be 2-32 characters".to_string()));
    }
    if !username.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err(AppError::BadRequest(
            "username may only contain letters, numbers, underscores, and hyphens".to_string(),
        ));
    }
    Ok(())
}

async fn issue_tokens(
    state: &AppState,
    jar: CookieJar,
    user_id: Uuid,
    username: &str,
    account_status: &str,
) -> crate::error::Result<(String, CookieJar)> {
    let jwt = tokens::encode_jwt(user_id, username, account_status, &state.config)?;
    let raw_token = tokens::generate_refresh_token();
    let token_hash = tokens::hash_refresh_token(&raw_token, &state.config.jwt_secret);
    let expires_at = OffsetDateTime::now_utc()
        + Duration::days(state.config.refresh_token_expiry_days as i64);

    sqlx::query(
        "INSERT INTO refresh_tokens (user_id, token_hash, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(&state.db)
    .await?;

    let cookie = Cookie::build(("refresh_token", raw_token))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::days(state.config.refresh_token_expiry_days as i64))
        .build();

    Ok((jwt, jar.add(cookie)))
}

// --- Register ---

#[derive(Deserialize)]
struct RegisterBody {
    username: String,
    email: String,
    password: String,
}

async fn register(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<RegisterBody>,
) -> crate::error::Result<(StatusCode, CookieJar, Json<AuthResponse>)> {
    validate_username(&body.username)?;

    if !body.email.contains('@') || body.email.len() < 3 {
        return Err(AppError::BadRequest("invalid email address".to_string()));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".to_string()));
    }

    match pwned::check_password(&state.http_client, &body.password).await {
        PwnedCheckResult::Pwned { count } => {
            return Err(AppError::BadRequest(format!(
                "This password has appeared in {} known data breach{}. \
                 Please choose a different password.",
                count,
                if count == 1 { "" } else { "es" }
            )));
        }
        PwnedCheckResult::ServiceUnavailable => {
            return Err(AppError::ServiceUnavailable);
        }
        PwnedCheckResult::Clean => {}
    }

    let password_hash = password::hash(&body.password)?;

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: Uuid,
        username: String,
        account_status: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "INSERT INTO users (username, email, password_hash)
         VALUES ($1, $2, $3)
         RETURNING id, username::TEXT as username, account_status",
    )
    .bind(&body.username)
    .bind(body.email.to_lowercase())
    .bind(password_hash)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if let sqlx::Error::Database(ref dbe) = e {
            let msg = dbe.message();
            if msg.contains("users_username_key") || msg.contains("username") {
                return AppError::Conflict("username already taken".to_string());
            }
            if msg.contains("users_email_key") || msg.contains("email") {
                return AppError::Conflict("email already registered".to_string());
            }
        }
        AppError::Database(e)
    })?;

    let (jwt, jar) =
        issue_tokens(&state, jar, user.id, &user.username, &user.account_status).await?;

    Ok((
        StatusCode::CREATED,
        jar,
        Json(AuthResponse {
            access_token: jwt,
            user: UserInfo {
                id: user.id,
                username: user.username,
                account_status: user.account_status,
            },
        }),
    ))
}

// --- Login ---

#[derive(Deserialize)]
struct LoginBody {
    identifier: String, // username or email
    password: String,
}

async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    jar: CookieJar,
    Json(body): Json<LoginBody>,
) -> crate::error::Result<(CookieJar, Json<AuthResponse>)> {
    let ip = crate::rate_limit::extract_client_ip(&headers, Some(addr));
    state.rate_limiters.login_ip.check_key(&ip)
        .map_err(|_| AppError::TooManyRequests)?;
    state.rate_limiters.login_identifier.check_key(&body.identifier.to_lowercase())
        .map_err(|_| AppError::TooManyRequests)?;

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: Uuid,
        username: String,
        password_hash: String,
        account_status: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, username::TEXT as username, password_hash, account_status
         FROM users
         WHERE username = $1 OR email = $1
         LIMIT 1",
    )
    .bind(&body.identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    if !password::verify(&body.password, &user.password_hash)? {
        return Err(AppError::Unauthorized);
    }

    if user.account_status == "compromised" {
        return Err(AppError::BadRequest(
            "account is locked — use TOTP or email OTP to unlock".to_string(),
        ));
    }

    let (jwt, jar) =
        issue_tokens(&state, jar, user.id, &user.username, &user.account_status).await?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: jwt,
            user: UserInfo {
                id: user.id,
                username: user.username,
                account_status: user.account_status,
            },
        }),
    ))
}

// --- Refresh ---

async fn refresh(
    State(state): State<AppState>,
    jar: CookieJar,
) -> crate::error::Result<(CookieJar, Json<serde_json::Value>)> {
    let raw_token = jar
        .get("refresh_token")
        .map(|c| c.value().to_string())
        .ok_or(AppError::Unauthorized)?;

    let token_hash = tokens::hash_refresh_token(&raw_token, &state.config.jwt_secret);

    #[derive(sqlx::FromRow)]
    struct TokenRow {
        id: Uuid,
        user_id: Uuid,
        revoked_at: Option<OffsetDateTime>,
        expires_at: OffsetDateTime,
    }

    let row = sqlx::query_as::<_, TokenRow>(
        "SELECT id, user_id, revoked_at, expires_at
         FROM refresh_tokens
         WHERE token_hash = $1",
    )
    .bind(&token_hash)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::Unauthorized)?;

    // Replay attack: token was already used
    if row.revoked_at.is_some() {
        // Kill all sessions and mark account compromised
        sqlx::query(
            "UPDATE refresh_tokens SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        )
        .bind(row.user_id)
        .execute(&state.db)
        .await?;

        sqlx::query(
            "UPDATE users SET account_status = 'compromised', compromise_detected_at = now()
             WHERE id = $1",
        )
        .bind(row.user_id)
        .execute(&state.db)
        .await?;

        return Err(AppError::Unauthorized);
    }

    if row.expires_at < OffsetDateTime::now_utc() {
        return Err(AppError::Unauthorized);
    }

    // Rotate: revoke current token
    sqlx::query("UPDATE refresh_tokens SET revoked_at = now() WHERE id = $1")
        .bind(row.id)
        .execute(&state.db)
        .await?;

    // Fetch current user state for new JWT
    #[derive(sqlx::FromRow)]
    struct UserRow {
        username: String,
        account_status: String,
    }
    let user = sqlx::query_as::<_, UserRow>(
        "SELECT username::TEXT as username, account_status FROM users WHERE id = $1",
    )
    .bind(row.user_id)
    .fetch_one(&state.db)
    .await?;

    let (jwt, jar) =
        issue_tokens(&state, jar, row.user_id, &user.username, &user.account_status).await?;

    Ok((jar, Json(serde_json::json!({ "access_token": jwt }))))
}

// --- Logout ---

async fn logout(
    State(state): State<AppState>,
    _auth: AuthUser,
    jar: CookieJar,
) -> crate::error::Result<(CookieJar, StatusCode)> {
    if let Some(cookie) = jar.get("refresh_token") {
        let raw_token = cookie.value().to_string();
        let token_hash = tokens::hash_refresh_token(&raw_token, &state.config.jwt_secret);
        sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
            .bind(&token_hash)
            .execute(&state.db)
            .await?;
    }

    // Clear cookie
    let expired = Cookie::build(("refresh_token", ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/api/auth/refresh")
        .max_age(Duration::seconds(0))
        .build();

    Ok((jar.add(expired), StatusCode::NO_CONTENT))
}

// --- TOTP Enrollment ---

async fn totp_enroll(
    State(state): State<AppState>,
    auth: AuthUser,
) -> crate::error::Result<Json<serde_json::Value>> {
    // Check if already enrolled
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM totp_secrets WHERE user_id = $1 AND verified_at IS NOT NULL",
    )
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if existing > 0 {
        return Err(AppError::Conflict("TOTP already enrolled".to_string()));
    }

    // Remove any pending (unverified) enrollment
    sqlx::query("DELETE FROM totp_secrets WHERE user_id = $1 AND verified_at IS NULL")
        .bind(auth.user_id)
        .execute(&state.db)
        .await?;

    let secret = totp::generate_secret();
    let encrypted = totp::encrypt_secret(&secret, &state.config.totp_encryption_key)?;
    let qr = totp::qr_url(&secret, &auth.username, &state.config.totp_issuer)?;

    sqlx::query(
        "INSERT INTO totp_secrets (user_id, secret_encrypted) VALUES ($1, $2)",
    )
    .bind(auth.user_id)
    .bind(&encrypted)
    .execute(&state.db)
    .await?;

    Ok(Json(serde_json::json!({ "qr_url": qr })))
}

#[derive(Deserialize)]
struct TotpCodeBody {
    code: String,
}

async fn totp_verify_enrollment(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<TotpCodeBody>,
) -> crate::error::Result<StatusCode> {
    state.rate_limiters.totp_user.check_key(&auth.user_id)
        .map_err(|_| AppError::TooManyRequests)?;
    #[derive(sqlx::FromRow)]
    struct SecretRow {
        id: Uuid,
        secret_encrypted: String,
    }

    let row = sqlx::query_as::<_, SecretRow>(
        "SELECT id, secret_encrypted FROM totp_secrets
         WHERE user_id = $1 AND verified_at IS NULL
         ORDER BY enrolled_at DESC LIMIT 1",
    )
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("no pending TOTP enrollment".to_string()))?;

    let secret = totp::decrypt_secret(&row.secret_encrypted, &state.config.totp_encryption_key)?;

    if !totp::verify_code(&secret, &body.code, &auth.username, &state.config.totp_issuer)? {
        return Err(AppError::BadRequest("invalid TOTP code".to_string()));
    }

    sqlx::query("UPDATE totp_secrets SET verified_at = now() WHERE id = $1")
        .bind(row.id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

// --- Unlock via TOTP ---

#[derive(Deserialize)]
struct UnlockTotpBody {
    identifier: String, // username or email
    code: String,
}

async fn unlock_totp(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<UnlockTotpBody>,
) -> crate::error::Result<(CookieJar, Json<AuthResponse>)> {
    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: Uuid,
        username: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, username::TEXT as username
         FROM users WHERE (username = $1 OR email = $1) AND account_status = 'compromised'",
    )
    .bind(&body.identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("no compromised account found with that identifier".to_string()))?;

    #[derive(sqlx::FromRow)]
    struct SecretRow {
        secret_encrypted: String,
    }

    let row = sqlx::query_as::<_, SecretRow>(
        "SELECT secret_encrypted FROM totp_secrets
         WHERE user_id = $1 AND verified_at IS NOT NULL
         ORDER BY verified_at DESC LIMIT 1",
    )
    .bind(user.id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest(
        "no verified TOTP enrolled — use email OTP to unlock".to_string(),
    ))?;

    let secret = totp::decrypt_secret(&row.secret_encrypted, &state.config.totp_encryption_key)?;

    if !totp::verify_code(&secret, &body.code, &user.username, &state.config.totp_issuer)? {
        return Err(AppError::BadRequest("invalid TOTP code".to_string()));
    }

    // Unlock
    sqlx::query(
        "UPDATE users SET account_status = 'active', compromise_detected_at = NULL WHERE id = $1",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let (jwt, jar) = issue_tokens(&state, jar, user.id, &user.username, "active").await?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: jwt,
            user: UserInfo {
                id: user.id,
                username: user.username,
                account_status: "active".to_string(),
            },
        }),
    ))
}

// --- Unlock via Email OTP ---

#[derive(Deserialize)]
struct EmailOtpSendBody {
    identifier: String,
}

async fn unlock_email_otp_send(
    State(state): State<AppState>,
    Json(body): Json<EmailOtpSendBody>,
) -> crate::error::Result<StatusCode> {
    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: Uuid,
        email: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, email FROM users
         WHERE (username = $1 OR email = $1) AND account_status = 'compromised'",
    )
    .bind(&body.identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("no compromised account found".to_string()))?;

    // Don't allow email OTP if TOTP is enrolled — use TOTP instead
    let totp_enrolled = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM totp_secrets WHERE user_id = $1 AND verified_at IS NOT NULL",
    )
    .bind(user.id)
    .fetch_one(&state.db)
    .await?;

    if totp_enrolled > 0 {
        return Err(AppError::BadRequest(
            "TOTP is enrolled — use TOTP to unlock".to_string(),
        ));
    }

    let otp = crate::auth::email::generate_otp();
    let redis_key = format!("email_otp:{}", user.id);

    // Store in Redis with 5-minute TTL
    let mut redis = state.redis.clone();
    redis::cmd("SET")
        .arg(&redis_key)
        .arg(&otp)
        .arg("EX")
        .arg(300u64)
        .query_async::<_, ()>(&mut redis)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis set: {e}")))?;

    crate::auth::email::send_otp(&user.email, &otp, &state.config).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct EmailOtpVerifyBody {
    identifier: String,
    code: String,
}

async fn unlock_email_otp_verify(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(body): Json<EmailOtpVerifyBody>,
) -> crate::error::Result<(CookieJar, Json<AuthResponse>)> {
    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: Uuid,
        username: String,
    }

    let user = sqlx::query_as::<_, UserRow>(
        "SELECT id, username::TEXT as username FROM users
         WHERE (username = $1 OR email = $1) AND account_status = 'compromised'",
    )
    .bind(&body.identifier)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::BadRequest("no compromised account found".to_string()))?;

    let redis_key = format!("email_otp:{}", user.id);
    let mut redis = state.redis.clone();

    let stored_otp: Option<String> = redis::cmd("GET")
        .arg(&redis_key)
        .query_async(&mut redis)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis get: {e}")))?;

    let stored = stored_otp.ok_or(AppError::BadRequest("OTP expired or not sent".to_string()))?;

    if stored != body.code {
        return Err(AppError::BadRequest("invalid OTP code".to_string()));
    }

    // Consume OTP
    redis::cmd("DEL")
        .arg(&redis_key)
        .query_async::<_, ()>(&mut redis)
        .await
        .map_err(|e| AppError::Internal(anyhow::anyhow!("redis del: {e}")))?;

    // Unlock
    sqlx::query(
        "UPDATE users SET account_status = 'active', compromise_detected_at = NULL WHERE id = $1",
    )
    .bind(user.id)
    .execute(&state.db)
    .await?;

    let (jwt, jar) = issue_tokens(&state, jar, user.id, &user.username, "active").await?;

    Ok((
        jar,
        Json(AuthResponse {
            access_token: jwt,
            user: UserInfo {
                id: user.id,
                username: user.username,
                account_status: "active".to_string(),
            },
        }),
    ))
}
