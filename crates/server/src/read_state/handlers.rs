use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize, Default)]
pub struct MarkReadBody {
    /// Newest message the client has seen. Omitted → mark read as of now().
    #[serde(default)]
    pub message_id: Option<Uuid>,
}

pub async fn mark_channel_read(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    body: Option<Json<MarkReadBody>>,
) -> Result<(), AppError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();

    let member: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_members sm \
         JOIN channels c ON c.server_id = sm.server_id \
         WHERE c.id = $1 AND sm.user_id = $2",
    )
    .bind(channel_id)
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await?;
    if member.is_none() {
        return Err(AppError::NotFound);
    }

    let ts: Option<DateTime<Utc>> = match body.message_id {
        Some(mid) => sqlx::query_scalar(
            "SELECT \"timestamp\" FROM messages WHERE id = $1 AND channel_id = $2",
        )
        .bind(mid)
        .bind(channel_id)
        .fetch_optional(&state.pool)
        .await?,
        None => None,
    };

    sqlx::query(
        "INSERT INTO channel_read_state (user_id, channel_id, last_read_at) \
         VALUES ($1, $2, COALESCE($3::timestamptz, now())) \
         ON CONFLICT (user_id, channel_id) \
         DO UPDATE SET last_read_at = GREATEST(channel_read_state.last_read_at, EXCLUDED.last_read_at)",
    )
    .bind(session.user_id)
    .bind(channel_id)
    .bind(ts)
    .execute(&state.pool)
    .await?;

    Ok(())
}

pub async fn mark_dm_read(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    body: Option<Json<MarkReadBody>>,
) -> Result<(), AppError> {
    let body = body.map(|Json(b)| b).unwrap_or_default();

    let member: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id = $2",
    )
    .bind(dm_id)
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await?;
    if member.is_none() {
        return Err(AppError::NotFound);
    }

    let ts: Option<DateTime<Utc>> = match body.message_id {
        Some(mid) => sqlx::query_scalar(
            "SELECT \"timestamp\" FROM dm_messages WHERE id = $1 AND dm_channel_id = $2",
        )
        .bind(mid)
        .bind(dm_id)
        .fetch_optional(&state.pool)
        .await?,
        None => None,
    };

    sqlx::query(
        "INSERT INTO dm_read_state (user_id, dm_channel_id, last_read_at) \
         VALUES ($1, $2, COALESCE($3::timestamptz, now())) \
         ON CONFLICT (user_id, dm_channel_id) \
         DO UPDATE SET last_read_at = GREATEST(dm_read_state.last_read_at, EXCLUDED.last_read_at)",
    )
    .bind(session.user_id)
    .bind(dm_id)
    .bind(ts)
    .execute(&state.pool)
    .await?;

    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChannelUnread {
    pub channel_id: Uuid,
    pub unread: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DmUnread {
    pub dm_channel_id: Uuid,
    pub unread: i64,
}

#[derive(Debug, Serialize)]
pub struct UnreadsResponse {
    pub channels: Vec<ChannelUnread>,
    pub dms: Vec<DmUnread>,
}

pub async fn get_unreads(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<UnreadsResponse>, AppError> {
    // First load ever: lazily baseline every channel/DM to "read as of now"
    // so a new account doesn't face a wall of thousands of unreads.
    sqlx::query(
        "INSERT INTO channel_read_state (user_id, channel_id) \
         SELECT $1, c.id FROM channels c \
         JOIN server_members sm ON sm.server_id = c.server_id AND sm.user_id = $1 \
         ON CONFLICT DO NOTHING",
    )
    .bind(session.user_id)
    .execute(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO dm_read_state (user_id, dm_channel_id) \
         SELECT $1, dcm.dm_channel_id FROM dm_channel_members dcm WHERE dcm.user_id = $1 \
         ON CONFLICT DO NOTHING",
    )
    .bind(session.user_id)
    .execute(&state.pool)
    .await?;

    let channels: Vec<ChannelUnread> = sqlx::query_as(
        "SELECT rs.channel_id, \
                count(m.id) FILTER ( \
                    WHERE m.\"timestamp\" > rs.last_read_at \
                      AND m.author_id IS DISTINCT FROM $1 \
                ) AS unread \
         FROM channel_read_state rs \
         JOIN server_members sm ON sm.user_id = rs.user_id \
         JOIN channels c ON c.id = rs.channel_id AND c.server_id = sm.server_id \
         LEFT JOIN messages m ON m.channel_id = rs.channel_id \
         WHERE rs.user_id = $1 \
         GROUP BY rs.channel_id, rs.last_read_at \
         HAVING count(m.id) FILTER ( \
                    WHERE m.\"timestamp\" > rs.last_read_at \
                      AND m.author_id IS DISTINCT FROM $1 \
                ) > 0",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    let dms: Vec<DmUnread> = sqlx::query_as(
        "SELECT rs.dm_channel_id, \
                count(m.id) FILTER ( \
                    WHERE m.\"timestamp\" > rs.last_read_at \
                      AND m.author_id IS DISTINCT FROM $1 \
                ) AS unread \
         FROM dm_read_state rs \
         JOIN dm_channel_members dcm ON dcm.user_id = rs.user_id AND dcm.dm_channel_id = rs.dm_channel_id \
         LEFT JOIN dm_messages m ON m.dm_channel_id = rs.dm_channel_id \
         WHERE rs.user_id = $1 \
         GROUP BY rs.dm_channel_id, rs.last_read_at \
         HAVING count(m.id) FILTER ( \
                    WHERE m.\"timestamp\" > rs.last_read_at \
                      AND m.author_id IS DISTINCT FROM $1 \
                ) > 0",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(UnreadsResponse { channels, dms }))
}
