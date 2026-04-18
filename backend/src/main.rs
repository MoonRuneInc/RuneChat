use runechat_backend::{api, config::Config, state::AppState};
use sqlx::postgres::PgPoolOptions;
use redis::aio::ConnectionManager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use dashmap::DashMap;

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
        .connect(&config.database_url)
        .await?;

    sqlx::migrate!("./migrations").run(&db).await?;

    let redis_client = redis::Client::open(config.redis_url.clone())?;
    let redis = ConnectionManager::new(redis_client).await?;

    let state = AppState {
        db: db.clone(),
        redis,
        config: config.clone(),
        ws_senders: Arc::new(DashMap::new()),
    };

    // Start Redis broker as a background task
    let broker_state = state.clone();
    tokio::spawn(async move {
        runechat_backend::realtime::broker::run(
            broker_state.config.redis_url,
            broker_state.db,
            broker_state.ws_senders,
        )
        .await;
    });

    let app = api::router()
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::info!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
