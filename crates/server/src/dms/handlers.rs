use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::social::{are_friends, ordered_pair, user_id_by_username};
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;
const MAX_CONTENT_LEN: usize = 4000;

async fn require_participant(
    pool: &sqlx::PgPool,
    dm_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_a, user_b FROM dm_channels WHERE id = $1")
            .bind(dm_id)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((a, b)) if a == user_id || b == user_id => Ok(()),
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::NotFound),
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DmChannelDto {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub peer_username: String,
}

pub async fn list_dms(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<DmChannelDto>>, AppError> {
    let dms: Vec<DmChannelDto> = sqlx::query_as(
        "SELECT dm_channels.id, \
                users.id AS peer_id, \
                users.username AS peer_username \
         FROM dm_channels \
         JOIN users ON users.id = CASE WHEN dm_channels.user_a = $1 \
                                        THEN dm_channels.user_b \
                                        ELSE dm_channels.user_a END \
         WHERE dm_channels.user_a = $1 OR dm_channels.user_b = $1 \
         ORDER BY dm_channels.created_at DESC",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(dms))
}

#[derive(Debug, Deserialize)]
pub struct OpenDmRequest {
    pub username: String,
}

pub async fn open_dm(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<OpenDmRequest>,
) -> Result<Json<DmChannelDto>, AppError> {
    let peer_id = user_id_by_username(&state.pool, payload.username.trim()).await?;
    if peer_id == session.user_id {
        return Err(AppError::InvalidName);
    }
    if !are_friends(&state.pool, session.user_id, peer_id).await? {
        return Err(AppError::Unauthorized);
    }

    let (user_a, user_b) = ordered_pair(session.user_id, peer_id);

    sqlx::query(
        "INSERT INTO dm_channels (user_a, user_b) VALUES ($1, $2) \
         ON CONFLICT (user_a, user_b) DO NOTHING",
    )
    .bind(user_a)
    .bind(user_b)
    .execute(&state.pool)
    .await?;

    let dm: DmChannelDto = sqlx::query_as(
        "SELECT dm_channels.id, $3::uuid AS peer_id, users.username AS peer_username \
         FROM dm_channels JOIN users ON users.id = $3 \
         WHERE dm_channels.user_a = $1 AND dm_channels.user_b = $2",
    )
    .bind(user_a)
    .bind(user_b)
    .bind(peer_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(dm))
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
    Path(dm_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    async fn timestamp_of(pool: &sqlx::PgPool, id: Uuid) -> Result<DateTime<Utc>, AppError> {
        let row: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT \"timestamp\" FROM dm_messages WHERE id = $1")
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
            "SELECT dm_messages.id, dm_messages.author_id, users.username AS author, \
                    dm_messages.encrypted_blob, dm_messages.\"timestamp\" \
             FROM dm_messages \
             LEFT JOIN users ON users.id = dm_messages.author_id \
             WHERE dm_messages.dm_channel_id = $1 \
               AND dm_messages.\"timestamp\" > $2 \
             ORDER BY dm_messages.\"timestamp\" ASC \
             LIMIT $3",
        )
        .bind(dm_id)
        .bind(after_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        let mut rows: Vec<MessageRow> = sqlx::query_as(
            "SELECT dm_messages.id, dm_messages.author_id, users.username AS author, \
                    dm_messages.encrypted_blob, dm_messages.\"timestamp\" \
             FROM dm_messages \
             LEFT JOIN users ON users.id = dm_messages.author_id \
             WHERE dm_messages.dm_channel_id = $1 \
               AND ($2::timestamptz IS NULL OR dm_messages.\"timestamp\" < $2) \
             ORDER BY dm_messages.\"timestamp\" DESC \
             LIMIT $3",
        )
        .bind(dm_id)
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
    Path(dm_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    let content = payload.content.trim();
    if content.is_empty() || content.chars().count() > MAX_CONTENT_LEN {
        return Err(AppError::InvalidMessage);
    }

    let row: MessageRow = sqlx::query_as(
        "INSERT INTO dm_messages (dm_channel_id, author_id, encrypted_blob) \
         VALUES ($1, $2, $3) \
         RETURNING id, author_id, NULL::text AS author, encrypted_blob, \"timestamp\"",
    )
    .bind(dm_id)
    .bind(session.user_id)
    .bind(content.as_bytes())
    .fetch_one(&state.pool)
    .await?;

    let mut dto = MessageDto::from(row);
    dto.author = session.username;

    Ok(Json(dto))
}
