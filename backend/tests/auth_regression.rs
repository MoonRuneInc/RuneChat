use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap, HeaderName, HeaderValue},
};
use base64::Engine;
use sqlx::PgPool;

/// Regression test: Rhea flagged that stale access tokens issued before a compromise
/// replay detection could still authorize protected routes. The AuthUser extractor
/// must query the current account_status from the DB, not trust the JWT claim.
#[sqlx::test(migrations = "./migrations")]
async fn stale_access_token_rejected_after_compromise(pool: PgPool) {
    // 1. Register a user
    let user_id: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO users (username, email, password_hash)
         VALUES ('alice', 'alice@example.com', 'hash') RETURNING id"
    )
    .fetch_one(&pool).await.unwrap();

    // 2. Issue an access JWT (simulating login)
    let config = runechat_backend::Config {
        database_url: String::new(),
        redis_url: String::new(),
        jwt_secret: "test-secret-32-bytes-min-length!!".to_string(),
        jwt_expiry_seconds: 900,
        refresh_token_expiry_days: 7,
        totp_issuer: "RuneChat".to_string(),
        totp_encryption_key: base64::engine::general_purpose::STANDARD.encode([0u8; 32]),
        domain: "localhost".to_string(),
        smtp: None,
    };
    let access_token = runechat_backend::auth::tokens::encode_jwt(
        user_id.0, "alice", "active", &config
    ).unwrap();

    // 3. Simulate replay detection marking the account compromised
    sqlx::query(
        "UPDATE users SET account_status = 'compromised', compromise_detected_at = now() WHERE id = $1"
    )
    .bind(user_id.0)
    .execute(&pool).await.unwrap();

    // 4. Build app state with real DB
    let redis_client = redis::Client::open("redis://127.0.0.1:6379").unwrap();
    let redis = redis::aio::ConnectionManager::new(redis_client).await.unwrap();
    let state = runechat_backend::AppState {
        db: pool.clone(),
        redis,
        config: config.clone(),
        ws_senders: std::sync::Arc::new(dashmap::DashMap::new()),
        rate_limiters: runechat_backend::rate_limit::RateLimiters::new(),
    };

    // 5. Construct request parts with the now-stale "active" JWT
    let mut headers = HeaderMap::new();
    headers.insert(
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {access_token}")).unwrap(),
    );
    let req = axum::http::Request::builder()
        .method(axum::http::Method::POST)
        .uri("/api/auth/totp/enroll")
        .body(())
        .unwrap();
    let (mut parts, _) = req.into_parts();
    parts.headers = headers;

    // 6. Attempt to extract AuthUser — must fail because DB says compromised
    let result = runechat_backend::auth::middleware::AuthUser::from_request_parts(
        &mut parts, &state,
    ).await;

    assert!(result.is_err(), "stale access token for compromised account must be rejected");
}
