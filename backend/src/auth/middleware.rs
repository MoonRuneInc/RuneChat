use super::tokens;
use crate::{error::AppError, state::AppState};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub username: String,
    pub account_status: String,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let claims = tokens::decode_jwt(token, &app_state.config)?;
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

        // Verify current account status from DB — stale JWTs may claim "active"
        // after a compromise replay detection invalidated the account.
        let row: (String,) = sqlx::query_as("SELECT account_status FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&app_state.db)
            .await?
            .ok_or(AppError::Unauthorized)?;

        let account_status = row.0;
        if account_status == "compromised" {
            return Err(AppError::Unauthorized);
        }

        Ok(AuthUser {
            user_id,
            username: claims.username,
            account_status,
        })
    }
}
