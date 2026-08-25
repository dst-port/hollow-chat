use axum::extract::{FromRef, FromRequestParts};
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;
use chrono::Utc;
use uuid::Uuid;

use crate::error::AppError;
use crate::session::hash_token;
use crate::state::AppState;

pub struct AuthSession {
    pub user_id: Uuid,
    pub username: String,
    pub session_id: Uuid,
}

#[derive(sqlx::FromRow)]
struct SessionRow {
    session_id: Uuid,
    user_id: Uuid,
    username: String,
}

pub async fn resolve_token(pool: &sqlx::PgPool, token: &str) -> Result<AuthSession, AppError> {
    let token_hash = hash_token(token);

    let row: Option<SessionRow> = sqlx::query_as(
        "SELECT sessions.id AS session_id, users.id AS user_id, users.username AS username \
         FROM sessions \
         JOIN users ON users.id = sessions.user_id \
         WHERE sessions.token_hash = $1 AND sessions.expires_at > $2",
    )
    .bind(&token_hash)
    .bind(Utc::now())
    .fetch_optional(pool)
    .await?;

    let row = row.ok_or(AppError::Unauthorized)?;

    Ok(AuthSession {
        user_id: row.user_id,
        username: row.username,
        session_id: row.session_id,
    })
}

impl<S> FromRequestParts<S> for AuthSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let header_token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        let token = match header_token {
            Some(token) => token.to_string(),
            None => parts
                .uri
                .query()
                .and_then(|query| {
                    query.split('&').find_map(|pair| {
                        let (key, value) = pair.split_once('=')?;
                        (key == "token").then(|| value.to_string())
                    })
                })
                .ok_or(AppError::Unauthorized)?,
        };

        resolve_token(&app_state.pool, &token).await
    }
}
