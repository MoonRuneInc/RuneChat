pub mod auth;
pub mod health;
pub mod invites;
pub mod servers;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health::health_check))
        .nest("/api/auth", auth::router())
        .nest("/api/servers", servers::router())
        .nest("/api/invite", invites::router())
}
