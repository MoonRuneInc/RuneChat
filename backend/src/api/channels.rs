use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;
use crate::{auth::middleware::AuthUser, error::AppError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        // Nested under /api/servers/:server_id/channels
        .route("/", post(create_channel).get(list_channels))
        .route("/:channel_id", get(get_channel).delete(delete_channel))
}

pub fn channel_router() -> Router<AppState> {
    // Mounted at /api/channels for direct channel access
    Router::new()
        .route("/:id", get(get_channel_by_id).delete(delete_channel_by_id))
}

// --- Slug generation ---

pub fn generate_slug(name: &str) -> String {
    use unicode_normalization::UnicodeNormalization;

    // NFKC normalize and lowercase
    let normalized: String = name
        .nfkc()
        .flat_map(|c| c.to_lowercase())
        .collect();

    // Replace spaces with hyphens, strip non-alphanumeric (except hyphen)
    let mut slug = String::with_capacity(normalized.len());
    let mut prev_hyphen = false;
    for c in normalized.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            prev_hyphen = false;
        } else if c == ' ' || c == '-' || c == '_' {
            if !prev_hyphen && !slug.is_empty() {
                slug.push('-');
                prev_hyphen = true;
            }
        }
        // All other characters (punctuation, emoji, etc.) are dropped
    }

    let slug = slug.trim_end_matches('-').to_string();

    if slug.is_empty() {
        format!("channel-{}", &Uuid::new_v4().to_string()[..8])
    } else {
        slug.chars().take(80).collect()
    }
}

// --- Types ---

#[derive(Serialize)]
struct ChannelResponse {
    id: Uuid,
    server_id: Uuid,
    display_name: String,
    slug: String,
    created_at: OffsetDateTime,
}

// --- Handlers ---

#[derive(Deserialize)]
struct CreateChannelBody {
    display_name: String,
}

fn is_unique_violation(e: &sqlx::Error) -> bool {
    matches!(e, sqlx::Error::Database(db_err) if db_err.constraint().is_some())
}

async fn create_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
    Json(body): Json<CreateChannelBody>,
) -> crate::error::Result<(StatusCode, Json<ChannelResponse>)> {
    let display_name = body.display_name.trim().to_string();
    if display_name.is_empty() || display_name.len() > 80 {
        return Err(AppError::BadRequest(
            "channel name must be 1-80 characters".to_string(),
        ));
    }

    // Validate requester is a member
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

    let base_slug = generate_slug(&display_name);
    let mut slug = resolve_slug_collision(&state, server_id, &base_slug).await?;
    let mut attempt = 0;

    let row = loop {
        match sqlx::query_as::<_, Row>(
            "INSERT INTO channels (server_id, display_name, slug)
             VALUES ($1, $2, $3)
             RETURNING id, created_at",
        )
        .bind(server_id)
        .bind(&display_name)
        .bind(&slug)
        .fetch_one(&state.db)
        .await
        {
            Ok(row) => break row,
            Err(ref e) if is_unique_violation(e) && attempt < 100 => {
                attempt += 1;
                slug = format!("{base_slug}-{}", attempt + 1);
                continue;
            }
            Err(e) => return Err(e.into()),
        }
    };

    Ok((
        StatusCode::CREATED,
        Json(ChannelResponse {
            id: row.id,
            server_id,
            display_name,
            slug,
            created_at: row.created_at,
        }),
    ))
}

async fn resolve_slug_collision(
    state: &AppState,
    server_id: Uuid,
    base_slug: &str,
) -> crate::error::Result<String> {
    let existing: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM channels WHERE server_id = $1 AND slug = $2",
    )
    .bind(server_id)
    .bind(base_slug)
    .fetch_one(&state.db)
    .await?;

    if existing == 0 {
        return Ok(base_slug.to_string());
    }

    for suffix in 2..=99u32 {
        let candidate = format!("{base_slug}-{suffix}");
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM channels WHERE server_id = $1 AND slug = $2",
        )
        .bind(server_id)
        .bind(&candidate)
        .fetch_one(&state.db)
        .await?;

        if count == 0 {
            return Ok(candidate);
        }
    }

    // Extremely unlikely — fall back to UUID suffix
    Ok(format!("{base_slug}-{}", &Uuid::new_v4().to_string()[..8]))
}

async fn list_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<Uuid>,
) -> crate::error::Result<Json<Vec<ChannelResponse>>> {
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
        display_name: String,
        slug: String,
        created_at: OffsetDateTime,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT id, display_name, slug, created_at
         FROM channels
         WHERE server_id = $1
         ORDER BY created_at ASC",
    )
    .bind(server_id)
    .fetch_all(&state.db)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|r| ChannelResponse {
                id: r.id,
                server_id,
                display_name: r.display_name,
                slug: r.slug,
                created_at: r.created_at,
            })
            .collect(),
    ))
}

async fn get_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, channel_id)): Path<(Uuid, Uuid)>,
) -> crate::error::Result<Json<ChannelResponse>> {
    get_channel_internal(&state, auth.user_id, server_id, channel_id).await
}

async fn get_channel_by_id(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> crate::error::Result<Json<ChannelResponse>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        server_id: Uuid,
    }
    let row = sqlx::query_as::<_, Row>("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    get_channel_internal(&state, auth.user_id, row.server_id, channel_id).await
}

async fn get_channel_internal(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    channel_id: Uuid,
) -> crate::error::Result<Json<ChannelResponse>> {
    let is_member = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_one(&state.db)
    .await?;

    if is_member == 0 {
        return Err(AppError::Forbidden);
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        display_name: String,
        slug: String,
        created_at: OffsetDateTime,
    }

    let row = sqlx::query_as::<_, Row>(
        "SELECT display_name, slug, created_at FROM channels WHERE id = $1 AND server_id = $2",
    )
    .bind(channel_id)
    .bind(server_id)
    .fetch_optional(&state.db)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(ChannelResponse {
        id: channel_id,
        server_id,
        display_name: row.display_name,
        slug: row.slug,
        created_at: row.created_at,
    }))
}

async fn delete_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, channel_id)): Path<(Uuid, Uuid)>,
) -> crate::error::Result<StatusCode> {
    delete_channel_internal(&state, auth.user_id, server_id, channel_id).await
}

async fn delete_channel_by_id(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<Uuid>,
) -> crate::error::Result<StatusCode> {
    #[derive(sqlx::FromRow)]
    struct Row {
        server_id: Uuid,
    }
    let row = sqlx::query_as::<_, Row>("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(&state.db)
        .await?
        .ok_or(AppError::NotFound)?;

    delete_channel_internal(&state, auth.user_id, row.server_id, channel_id).await
}

async fn delete_channel_internal(
    state: &AppState,
    user_id: Uuid,
    server_id: Uuid,
    channel_id: Uuid,
) -> crate::error::Result<StatusCode> {
    let role: Option<String> = sqlx::query_scalar(
        "SELECT role FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(&state.db)
    .await?;

    match role.as_deref() {
        Some("owner") | Some("admin") => {}
        Some(_) => return Err(AppError::Forbidden),
        None => return Err(AppError::Forbidden),
    }

    let deleted = sqlx::query(
        "DELETE FROM channels WHERE id = $1 AND server_id = $2",
    )
    .bind(channel_id)
    .bind(server_id)
    .execute(&state.db)
    .await?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_basic_cases() {
        assert_eq!(generate_slug("General Discussion"), "general-discussion");
        assert_eq!(generate_slug("Dev — Backend"), "dev-backend");
        assert_eq!(generate_slug("Off Topic"), "off-topic");
        assert_eq!(generate_slug("  leading spaces  "), "leading-spaces");
    }

    #[test]
    fn slug_collapses_consecutive_separators() {
        assert_eq!(generate_slug("hello   world"), "hello-world");
        assert_eq!(generate_slug("a---b"), "a-b");
    }

    #[test]
    fn slug_truncates_to_80() {
        let long = "a".repeat(100);
        assert_eq!(generate_slug(&long).len(), 80);
    }

    #[test]
    fn slug_empty_fallback_starts_with_channel() {
        let slug = generate_slug("---");
        assert!(slug.starts_with("channel-"), "got: {slug}");
    }

    #[test]
    fn slug_unicode_normalization() {
        // NFKC: ﬁ (fi ligature) → fi
        assert_eq!(generate_slug("ﬁnance"), "finance");
    }
}
