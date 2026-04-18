pub mod auth;
pub mod channels;
pub mod health;
pub mod invites;
pub mod messages;
pub mod servers;

use axum::Router;
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", axum::routing::get(health::health_check))
        .route("/ws", axum::routing::get(crate::realtime::ws::ws_handler))
        .nest("/api/auth", auth::router())
        .nest("/api/servers", servers::router())
        .nest("/api/servers/:server_id/channels", channels::router())
        .nest("/api/channels", channels::channel_router())
        .nest("/api/channels/:channel_id/messages", messages::router())
        .nest("/api/invite", invites::router())
}
