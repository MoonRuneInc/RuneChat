use crate::{auth::middleware::AuthUser, error::AppError, state::AppState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", post(create_server).get(list_servers))
        .route("/:id", get(get_server).delete(delete_server))
        .route("/:id/members", get(list_members))
        .route("/:id/members/:user_id", delete(kick_member))
}

// --- Types ---

#[derive(Serialize)]
struct ServerResponse {
    id: Uuid,
    name: String,
    owner_id: Uuid,
    member_count: i64,
    my_role: String,
}

#[derive(Serialize)]
struct MemberResponse {
    user_id: Uuid,
    username: String,
    role: String,
    joined_at: time::OffsetDateTime,
}

// --- Handlers ---

#[derive(Deserialize)]
struct CreateServerBody {
    name: String,
}

async fn create_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<CreateServerBody>,
) -> crate::error::Result<(StatusCode, Json<ServerResponse>)> {
    let name = body.name.trim().to_string();
    if name.is_empty() || name.len() > 100 {
        return Err(AppError::BadRequest(
            "server name must be 1-100 characters".to_string(),
        ));
    }

    let mut tx = state.db.begin().await?;

    let server_id: Uuid =
        sqlx::query_scalar("INSERT INTO servers (name, owner_id) VALUES ($1, $2) RETURNING id")
            .bind(&name)
            .bind(auth.user_id)
            .fetch_one(&mut *tx)
            .await?;

    // Add creator as owner in server_members
    sqlx::query("INSERT INTO server_members (server_id, user_id, role) VALUES ($1, $2, 'owner')")
        .bind(server_id)
        .bind(auth.user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok((
        StatusCode::CREATED,
        Json(ServerResponse {
            id: server_id,
            name,
            owner_id: auth.user_id,
            member_count: 1,
            my_role: "owner".to_string(),
        }),
    ))
}

async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> crate::error::Result<Json<Vec<ServerResponse>>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        name: String,
        owner_id: Uuid,
        member_count: i64,
        my_role: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            s.id,
            s.name,
            s.owner_id,
            (SELECT COUNT(*) FROM server_members sm2 WHERE sm2.server_id = s.id) AS member_count,
            sm.role AS my_role
        FROM servers s
        JOIN server_members sm ON sm.server_id = s.id AND sm.user_id = $1
        ORDER BY s.created_at ASC
        "#,
    )
    .bind(auth.user_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ServerResponse {
                id: r.id,
                name: r.name,
                owner_id: r.owner_id,
                member_count: r.member_count,
                my_role: r.my_role,
            })
            .collect(),
    ))
}

async fn get_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> crate::error::Result<Json<ServerResponse>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        name: String,
        owner_id: Uuid,
        member_count: i64,
        my_role: Option<String>,
    }

    let row = sqlx::query_as::<_, Row>(
        r#"
        SELECT
            s.name,
            s.owner_id,
            (SELECT COUNT(*) FROM server_members sm2 WHERE sm2.server_id = s.id) AS member_count,
            sm.role AS my_role
        FROM servers s
        LEFT JOIN server_members sm ON sm.server_id = s.id AND sm.user_id = $2
        WHERE s.id = $1
        "#,
    )
    .bind(server_id)
    .bind(auth.user_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    row.my_role.as_ref().ok_or(AppError::Forbidden)?;

    Ok(Json(ServerResponse {
        id: server_id,
        name: row.name,
        owner_id: row.owner_id,
        member_count: row.member_count,
        my_role: row.my_role.unwrap(),
    }))
}

async fn delete_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> crate::error::Result<StatusCode> {
    let role: Option<String> =
        sqlx::query_scalar("SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2")
            .bind(server_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;

    match role.as_deref() {
        Some("owner") => {}
        Some(_) => return Err(AppError::Forbidden),
        None => return Err(AppError::NotFound),
    }

    sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(server_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> crate::error::Result<Json<Vec<MemberResponse>>> {
    // Verify requester is a member
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
        user_id: Uuid,
        username: String,
        role: String,
        joined_at: time::OffsetDateTime,
    }

    let rows = sqlx::query_as::<_, Row>(
        r#"
        SELECT sm.user_id, u.username::TEXT as username, sm.role, sm.joined_at
        FROM server_members sm
        JOIN users u ON u.id = sm.user_id
        WHERE sm.server_id = $1
        ORDER BY sm.joined_at ASC
        "#,
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| MemberResponse {
                user_id: r.user_id,
                username: r.username,
                role: r.role,
                joined_at: r.joined_at,
            })
            .collect(),
    ))
}

async fn kick_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, target_user_id)): Path<(Uuid, Uuid)>,
) -> crate::error::Result<StatusCode> {
    let requester_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2")
            .bind(server_id)
            .bind(auth.user_id)
            .fetch_optional(&state.db)
            .await?;

    match requester_role.as_deref() {
        Some("owner") | Some("admin") => {}
        _ => return Err(AppError::Forbidden),
    }

    let target_role: Option<String> =
        sqlx::query_scalar("SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2")
            .bind(server_id)
            .bind(target_user_id)
            .fetch_optional(&state.db)
            .await?;

    match target_role.as_deref() {
        None => return Err(AppError::NotFound),
        Some("owner") => return Err(AppError::Forbidden), // cannot kick owner
        _ => {}
    }

    // Admin cannot kick another admin — only owner can
    if target_role.as_deref() == Some("admin") && requester_role.as_deref() != Some("owner") {
        return Err(AppError::Forbidden);
    }

    sqlx::query("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(target_user_id)
        .execute(&state.db)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    // Server endpoint integration tests are validated by Rhea during QA.
    // Key behaviors to verify:
    // - POST /api/servers creates server + member row (role=owner)
    // - GET /api/servers returns only servers the user is a member of
    // - DELETE /api/servers/:id requires owner role
    // - GET /api/servers/:id returns 403 for non-members
    // - kick_member: admin cannot kick owner; admin cannot kick another admin
}
