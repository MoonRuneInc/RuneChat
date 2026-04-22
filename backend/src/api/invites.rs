use crate::{auth::middleware::AuthUser, error::AppError, state::AppState};
use axum::{
    extract::{ConnectInfo, Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use time::OffsetDateTime;
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_invite))
        .route("/:code", get(preview_invite))
        .route("/:code/join", post(join_via_invite))
}

fn generate_invite_code() -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(8)
        .map(char::from)
        .collect()
}

// --- Types ---

#[derive(Serialize)]
struct InviteResponse {
    id: Uuid,
    server_id: Uuid,
    code: String,
    max_uses: Option<i32>,
    uses: i32,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

// --- Handlers ---

#[derive(Deserialize)]
struct CreateInviteBody {
    server_id: Uuid,
    max_uses: Option<i32>,
    expires_in_hours: Option<i64>,
}

async fn create_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateInviteBody>,
) -> crate::error::Result<(StatusCode, Json<InviteResponse>)> {
    // Verify requester is a member (any role can create invites in MVP)
    let is_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(body.server_id)
    .bind(auth.user_id)
    .fetch_one(&state.db)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    if let Some(max) = body.max_uses {
        if max < 1 {
            return Err(AppError::BadRequest(
                "max_uses must be at least 1".to_string(),
            ));
        }
    }

    let expires_at = body
        .expires_in_hours
        .map(|hours| OffsetDateTime::now_utc() + time::Duration::hours(hours));

    // Generate unique code (retry on collision — extremely rare with 8-char random)
    let code = loop {
        let candidate = generate_invite_code();
        let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM invites WHERE code = $1")
            .bind(&candidate)
            .fetch_one(&state.db)
            .await?;
        if exists == 0 {
            break candidate;
        }
    };

    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        created_at: OffsetDateTime,
    }

    let row = sqlx::query_as::<_, Row>(
        "INSERT INTO invites (server_id, creator_id, code, max_uses, expires_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, created_at",
    )
    .bind(body.server_id)
    .bind(auth.user_id)
    .bind(&code)
    .bind(body.max_uses)
    .bind(expires_at)
    .fetch_one(&state.db)
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            id: row.id,
            server_id: body.server_id,
            code,
            max_uses: body.max_uses,
            uses: 0,
            expires_at,
            created_at: row.created_at,
        }),
    ))
}

async fn preview_invite(
    State(state): State<AppState>,
    Path(code): Path<String>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> crate::error::Result<Json<serde_json::Value>> {
    let ip = crate::rate_limit::extract_client_ip(&headers, Some(addr));
    state
        .rate_limiters
        .invite_preview_ip
        .check_key(&ip)
        .map_err(|_| AppError::TooManyRequests)?;
    // Public endpoint — no auth required
    #[derive(sqlx::FromRow)]
    struct Row {
        server_name: String,
        member_count: i64,
        max_uses: Option<i32>,
        uses: i32,
        expires_at: Option<OffsetDateTime>,
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            s.name AS server_name,
            (SELECT COUNT(*) FROM server_members sm WHERE sm.server_id = s.id) AS member_count,
            i.max_uses,
            i.uses,
            i.expires_at
        FROM invites i
        JOIN servers s ON s.id = i.server_id
        WHERE i.code = $1
        "#,
    )
    .bind(&code)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    // Check validity
    let expired = row
        .expires_at
        .map(|e| e < OffsetDateTime::now_utc())
        .unwrap_or(false);
    let exhausted = row.max_uses.map(|m| row.uses >= m).unwrap_or(false);

    if expired || exhausted {
        return Err(AppError::NotFound);
    }

    Ok(Json(serde_json::json!({
        "server_name": row.server_name,
        "member_count": row.member_count,
        "valid": true,
    })))
}

async fn join_via_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(code): Path<String>,
    headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> crate::error::Result<Json<serde_json::Value>> {
    let ip = crate::rate_limit::extract_client_ip(&headers, Some(addr));
    state
        .rate_limiters
        .invite_join_ip
        .check_key(&ip)
        .map_err(|_| AppError::TooManyRequests)?;
    // Atomic check-and-increment using SELECT FOR UPDATE inside a transaction
    let mut tx = state.db.begin().await?;

    #[derive(sqlx::FromRow)]
    struct InviteRow {
        id: Uuid,
        server_id: Uuid,
        max_uses: Option<i32>,
        uses: i32,
        expires_at: Option<OffsetDateTime>,
    }

    let invite = sqlx::query_as::<_, InviteRow>(
        "SELECT id, server_id, max_uses, uses, expires_at
         FROM invites
         WHERE code = $1
         FOR UPDATE",
    )
    .bind(&code)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or(AppError::NotFound)?;

    // Validate invite
    if let Some(exp) = invite.expires_at {
        if exp < OffsetDateTime::now_utc() {
            return Err(AppError::BadRequest("invite has expired".to_string()));
        }
    }
    if let Some(max) = invite.max_uses {
        if invite.uses >= max {
            return Err(AppError::BadRequest(
                "invite has reached its use limit".to_string(),
            ));
        }
    }

    // Check if already a member
    let already_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(invite.server_id)
    .bind(auth.user_id)
    .fetch_one(&mut *tx)
    .await?;

    if already_member > 0 {
        tx.rollback().await?;
        return Err(AppError::Conflict(
            "already a member of this server".to_string(),
        ));
    }

    // Increment uses
    sqlx::query("UPDATE invites SET uses = uses + 1 WHERE id = $1")
        .bind(invite.id)
        .execute(&mut *tx)
        .await?;

    // Add member
    sqlx::query("INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'member')")
        .bind(invite.server_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    // Return server info
    let server_name: String = sqlx::query_scalar("SELECT name FROM servers WHERE id = $1")
        .bind(invite.server_id)
        .fetch_one(&state.db)
        .await?;

    Ok(Json(serde_json::json!({
        "server_id": invite.server_id,
        "server_name": server_name,
    })))
}

#[cfg(test)]
mod tests {
    // Invite endpoint integration tests validated by Rhea during QA.
    // Key behaviors to verify:
    // - Invite creation with max_uses and expires_at
    // - Preview returns 404 for expired or exhausted invites
    // - Join atomically increments uses; two concurrent joins to a max_uses=1 invite → only one succeeds
    // - Already-member join attempt returns 409
}
