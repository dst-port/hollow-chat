use axum::extract::{Query, State};
use axum::Json;
use serde::{Deserialize, Serialize};

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 5;
const MAX_LIMIT: i64 = 20;

fn validate_emoji(emoji: &str) -> Result<(), AppError> {
    let len_ok = (1..=32).contains(&emoji.chars().count());
    let no_whitespace = !emoji.chars().any(|c| c.is_whitespace() || c.is_control());
    if len_ok && no_whitespace {
        Ok(())
    } else {
        Err(AppError::InvalidEmoji)
    }
}

#[derive(Debug, Deserialize)]
pub struct RecordUseRequest {
    pub emoji: String,
}

pub async fn record_use(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<RecordUseRequest>,
) -> Result<(), AppError> {
    validate_emoji(&payload.emoji)?;

    sqlx::query(
        "INSERT INTO emoji_usage (user_id, emoji, count, last_used_at) \
         VALUES ($1, $2, 1, now()) \
         ON CONFLICT (user_id, emoji) \
         DO UPDATE SET count = emoji_usage.count + 1, last_used_at = now()",
    )
    .bind(session.user_id)
    .bind(&payload.emoji)
    .execute(&state.pool)
    .await?;

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct FrequentQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FrequentEmoji {
    pub emoji: String,
    pub count: i32,
}

pub async fn frequent(
    State(state): State<AppState>,
    session: AuthSession,
    Query(query): Query<FrequentQuery>,
) -> Result<Json<Vec<FrequentEmoji>>, AppError> {
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    let rows: Vec<FrequentEmoji> = sqlx::query_as(
        "SELECT emoji, count FROM emoji_usage \
         WHERE user_id = $1 \
         ORDER BY count DESC, last_used_at DESC \
         LIMIT $2",
    )
    .bind(session.user_id)
    .bind(limit)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}
