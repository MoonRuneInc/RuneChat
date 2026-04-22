use axum::extract::DefaultBodyLimit;
use axum::http::{header, HeaderName, HeaderValue};
use cauldron_backend::{api, config::Config, rate_limit::RateLimiters, state::AppState};
use dashmap::DashMap;
use redis::aio::ConnectionManager;
use reqwest;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::set_header::SetResponseHeaderLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    let db = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .max_lifetime(Duration::from_secs(1800))
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = ConnectionManager::new(redis_client).await?;

    let http_client = reqwest::Client::builder()
        .user_agent("Cauldron/1.0 (pwned-password-check)")
        .build()
        .expect("failed to build HTTP client");

    let state = AppState {
        db: db.clone(),
        redis,
        config: config.clone(),
        ws_senders: Arc::new(DashMap::new()),
        rate_limiters: RateLimiters::new(),
        http_client,
    };

    let broker_state = state.clone();
    tokio::spawn(async move {
        cauldron_backend::realtime::broker::run(
            broker_state.config.redis_url,
            broker_state.db,
            broker_state.ws_senders,
        )
        .await;
    });

    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(
            |origin: &axum::http::HeaderValue, _request_head: &axum::http::request::Parts| {
                let s = origin.as_bytes();
                s == b"https://chat.moonrune.cc"
                    || s.starts_with(b"http://localhost")
                    || s.starts_with(b"https://localhost")
                    || s.starts_with(b"https://tauri.localhost")
                    || s.starts_with(b"http://tauri.localhost")
            },
        ))
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::PATCH,
            axum::http::Method::DELETE,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .allow_credentials(true);

    let app = api::router()
        .with_state(state)
        .layer(DefaultBodyLimit::max(1024 * 1024)) // 1MB
        .layer(cors)
        .layer(tower_http::trace::TraceLayer::new_for_http())
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("referrer-policy"),
            HeaderValue::from_static("strict-origin-when-cross-origin"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        ));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
