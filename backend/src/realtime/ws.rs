use crate::{auth::tokens, state::AppState};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::{header::ORIGIN, HeaderValue, StatusCode},
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct WsQuery {
    token: String,
}

pub async fn ws_handler(
    State(state): State<AppState>,
    Query(params): Query<WsQuery>,
    ws: WebSocketUpgrade,
    headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    // Validate Origin header to block cross-site WebSocket hijacking
    let allowed_origin = HeaderValue::from_str(&format!("https://{}", state.config.domain))
        .unwrap_or_else(|_| HeaderValue::from_static("https://chat.moonrune.cc"));

    // Also allow localhost for development
    let origin = headers.get(ORIGIN);
    let origin_ok = match origin {
        None => true, // No origin = non-browser client (CLI tools, tests) — allow
        Some(o) => {
            o == &allowed_origin
                || o == HeaderValue::from_static("http://localhost:5173")
                || o == HeaderValue::from_static("http://localhost:3000")
        }
    };

    if !origin_ok {
        return (StatusCode::FORBIDDEN, "forbidden origin").into_response();
    }

    // Validate JWT from query param
    let claims = match tokens::decode_jwt(&params.token, &state.config) {
        Ok(c) => c,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
    };

    // Rhea fix: check live DB status — don't trust stale JWT claim
    let db_status: Option<String> =
        match sqlx::query_scalar("SELECT account_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.db)
            .await
        {
            Ok(s) => s,
            Err(_) => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
        };

    match db_status.as_deref() {
        Some("compromised") => {
            return (StatusCode::UNAUTHORIZED, "account compromised").into_response()
        }
        None => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
        _ => {}
    }

    ws.on_upgrade(move |socket| handle_socket(socket, state, user_id))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid) {
    let (mut sender, mut receiver) = socket.split();

    // Create an mpsc channel — broker writes to tx, we read from rx and forward to WS
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    // Register this connection
    state.ws_senders.insert(user_id, tx);

    tracing::info!("ws: user {user_id} connected");

    // Task 1: forward broker messages → WebSocket
    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(Message::Text(msg)).await.is_err() {
                break;
            }
        }
    });

    // Task 2: read from WebSocket (heartbeat / client messages, discard for MVP)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Ping(_) => {} // axum handles pong automatically
                _ => {}                // No client→server messages in MVP
            }
        }
    });

    // Wait for either task to finish (disconnect from either side)
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }

    // Deregister
    state.ws_senders.remove(&user_id);
    tracing::info!("ws: user {user_id} disconnected");
}
