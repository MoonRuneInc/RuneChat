use std::sync::Arc;
use dashmap::DashMap;
use sqlx::PgPool;
use redis::aio::ConnectionManager;
use tokio::sync::mpsc;
use uuid::Uuid;
use crate::config::Config;
use crate::rate_limit::RateLimiters;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub config: Config,
    pub ws_senders: Arc<DashMap<Uuid, mpsc::UnboundedSender<String>>>,
    pub rate_limiters: RateLimiters,
    pub http_client: reqwest::Client,
}
