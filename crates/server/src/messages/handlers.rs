use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;
const MAX_CONTENT_LEN: usize = 4000;

async fn require_channel_member(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as(
        "SELECT server_id FROM channels WHERE id = $1",
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await?;

    let (server_id,) = row.ok_or(AppError::NotFound)?;

    let member: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    member.map(|_| ()).ok_or(AppError::Unauthorized)
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id: Uuid,
    pub author_id: Option<Uuid>,
    pub author: String,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    author_id: Option<Uuid>,
    author: Option<String>,
    encrypted_blob: Vec<u8>,
    timestamp: DateTime<Utc>,
}

impl From<MessageRow> for MessageDto {
    fn from(row: MessageRow) -> Self {
        MessageDto {
            id: row.id,
            author_id: row.author_id,
            author: row.author.unwrap_or_else(|| "unknown".to_string()),
            content: String::from_utf8_lossy(&row.encrypted_blob).into_owned(),
            timestamp: row.timestamp,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    async fn timestamp_of(pool: &sqlx::PgPool, id: Uuid) -> Result<DateTime<Utc>, AppError> {
        let row: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT \"timestamp\" FROM messages WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?;
        Ok(row.ok_or(AppError::NotFound)?.0)
    }

    let before_ts = match query.before {
        Some(id) => Some(timestamp_of(&state.pool, id).await?),
        None => None,
    };
    let after_ts = match query.after {
        Some(id) => Some(timestamp_of(&state.pool, id).await?),
        None => None,
    };

    let rows: Vec<MessageRow> = if after_ts.is_some() {
        sqlx::query_as(
            "SELECT messages.id, messages.author_id, users.username AS author, \
                    messages.encrypted_blob, messages.\"timestamp\" \
             FROM messages \
             LEFT JOIN users ON users.id = messages.author_id \
             WHERE messages.channel_id = $1 \
               AND messages.\"timestamp\" > $2 \
             ORDER BY messages.\"timestamp\" ASC \
             LIMIT $3",
        )
        .bind(channel_id)
        .bind(after_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        let mut rows: Vec<MessageRow> = sqlx::query_as(
            "SELECT messages.id, messages.author_id, users.username AS author, \
                    messages.encrypted_blob, messages.\"timestamp\" \
             FROM messages \
             LEFT JOIN users ON users.id = messages.author_id \
             WHERE messages.channel_id = $1 \
               AND ($2::timestamptz IS NULL OR messages.\"timestamp\" < $2) \
             ORDER BY messages.\"timestamp\" DESC \
             LIMIT $3",
        )
        .bind(channel_id)
        .bind(before_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;
        rows.reverse();
        rows
    };

    let messages: Vec<MessageDto> = rows.into_iter().map(MessageDto::from).collect();

    Ok(Json(messages))
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

pub async fn send_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let content = payload.content.trim();
    if content.is_empty() || content.chars().count() > MAX_CONTENT_LEN {
        return Err(AppError::InvalidMessage);
    }

    let row: MessageRow = sqlx::query_as(
        "INSERT INTO messages (channel_id, author_id, encrypted_blob) \
         VALUES ($1, $2, $3) \
         RETURNING id, author_id, NULL::text AS author, encrypted_blob, \"timestamp\"",
    )
    .bind(channel_id)
    .bind(session.user_id)
    .bind(content.as_bytes())
    .fetch_one(&state.pool)
    .await?;

    let mut dto = MessageDto::from(row);
    dto.author = session.username;

    Ok(Json(dto))
}
