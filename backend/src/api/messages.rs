use crate::{auth::middleware::AuthUser, error::AppError, state::AppState};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new().route("/", post(send_message).get(get_messages))
}

// --- Types ---

#[derive(Serialize)]
struct MessageResponse {
    id: Uuid,
    channel_id: Uuid,
    author_id: Uuid,
    author_username: String,
    author_status: String,
    content: String,
    compromised_at_send: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    edited_at: Option<OffsetDateTime>,
}

// --- Handlers ---

#[derive(Deserialize)]
struct SendMessageBody {
    content: String,
}

async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Json(body): Json<SendMessageBody>,
) -> crate::error::Result<(StatusCode, Json<MessageResponse>)> {
    // Compromised accounts cannot send messages
    if auth.account_status == "compromised" {
        return Err(AppError::Forbidden);
    }

    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err(AppError::BadRequest(
            "message content cannot be empty".to_string(),
        ));
    }
    if content.chars().count() > 4000 {
        return Err(AppError::BadRequest(
            "message content must be 4000 characters or fewer".to_string(),
        ));
    }

    // Verify channel exists and user is a member of its server
    let server_id: Option<Uuid> =
        sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(&state.db)
            .await?;

    let server_id = server_id.ok_or(AppError::NotFound)?;

    let is_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        created_at: OffsetDateTime,
    }

    let row = sqlx::query_as::<_, Row>(
        "INSERT INTO messages (channel_id, author_id, content, compromised_at_send)
         VALUES ($1, $2, $3, false)
         RETURNING id, created_at",
    )
    .bind(channel_id)
    .bind(auth.user_id)
    .bind(&content)
    .fetch_one(&state.db)
    .await?;

    let msg = MessageResponse {
        id: row.id,
        channel_id,
        author_id: auth.user_id,
        author_username: auth.username.clone(),
        author_status: auth.account_status.clone(),
        content: content.clone(),
        compromised_at_send: false,
        created_at: row.created_at,
        edited_at: None,
    };

    // Publish to Redis for real-time delivery (Plan 6 consumes this)
    let payload = serde_json::json!({
        "type": "message.created",
        "channel_id": channel_id,
        "server_id": server_id,
        "message": {
            "id": msg.id,
            "author_id": msg.author_id,
            "author_username": msg.author_username,
            "author_status": msg.author_status,
            "content": msg.content,
            "compromised_at_send": msg.compromised_at_send,
            "created_at": msg.created_at.format(&time::format_description::well_known::Rfc3339).unwrap(),
        }
    });

    let redis_channel = format!("channel:{channel_id}");
    let mut redis = state.redis.clone();
    if let Err(e) = redis::cmd("PUBLISH")
        .arg(&redis_channel)
        .arg(payload.to_string())
        .query_async::<_, i64>(&mut redis)
        .await
    {
        // Log but don't fail the request — message is durably stored in DB
        tracing::warn!("redis publish failed for channel {channel_id}: {e}");
    }

    Ok((StatusCode::CREATED, Json(msg)))
}

#[derive(Deserialize)]
struct GetMessagesQuery {
    before: Option<Uuid>,
    limit: Option<i64>,
}

async fn get_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
    Query(params): Query<GetMessagesQuery>,
) -> crate::error::Result<Json<Vec<MessageResponse>>> {
    // Verify channel exists and user is a member
    let server_id: Option<Uuid> =
        sqlx::query_scalar("SELECT server_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(&state.db)
            .await?;

    let server_id = server_id.ok_or(AppError::NotFound)?;

    let is_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    let limit = params.limit.unwrap_or(50).clamp(1, 100);

    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        author_id: Uuid,
        author_username: String,
        author_status: String,
        content: String,
        compromised_at_send: bool,
        created_at: OffsetDateTime,
        edited_at: Option<OffsetDateTime>,
    }

    let rows = if let Some(before_id) = params.before {
        // Cursor-based pagination: messages before a given message ID
        let before_ts: Option<OffsetDateTime> =
            sqlx::query_scalar("SELECT created_at FROM messages WHERE id = $1 AND channel_id = $2")
                .bind(before_id)
                .bind(channel_id)
                .fetch_optional(&state.db)
                .await?;

        let before_ts = before_ts.ok_or(AppError::NotFound)?;

        sqlx::query_as::<_, Row>(
            r#"
            SELECT
                m.id,
                m.author_id,
                u.username::TEXT AS author_username,
                u.account_status AS author_status,
                m.content,
                m.compromised_at_send,
                m.created_at,
                m.edited_at
            FROM messages m
            JOIN users u ON u.id = m.author_id
            WHERE m.channel_id = $1 AND m.created_at < $2
            ORDER BY m.created_at DESC
            LIMIT $3
            "#,
        )
        .bind(channel_id)
        .bind(before_ts)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    } else {
        sqlx::query_as::<_, Row>(
            r#"
            SELECT
                m.id,
                m.author_id,
                u.username::TEXT AS author_username,
                u.account_status AS author_status,
                m.content,
                m.compromised_at_send,
                m.created_at,
                m.edited_at
            FROM messages m
            JOIN users u ON u.id = m.author_id
            WHERE m.channel_id = $1
            ORDER BY m.created_at DESC
            LIMIT $2
            "#,
        )
        .bind(channel_id)
        .bind(limit)
        .fetch_all(&state.db)
        .await?
    };

    // Return in chronological order (oldest first)
    let mut messages: Vec<MessageResponse> = rows
        .into_iter()
        .map(|r| MessageResponse {
            id: r.id,
            channel_id,
            author_id: r.author_id,
            author_username: r.author_username,
            author_status: r.author_status,
            content: r.content,
            compromised_at_send: r.compromised_at_send,
            created_at: r.created_at,
            edited_at: r.edited_at,
        })
        .collect();
    messages.sort_by_key(|m| m.created_at);
    Ok(Json(messages))
}
