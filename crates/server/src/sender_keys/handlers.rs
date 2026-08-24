use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

async fn require_channel_member(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
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

#[derive(Debug, Deserialize)]
pub struct SenderKeyEntry {
    pub recipient_id: Uuid,
    pub ciphertext: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishSenderKeysRequest {
    pub entries: Vec<SenderKeyEntry>,
}

pub async fn publish_sender_keys(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<PublishSenderKeysRequest>,
) -> Result<(), AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    if payload.entries.len() > 500 {
        return Err(AppError::InvalidMessage);
    }

    let mut tx = state.pool.begin().await?;
    for entry in &payload.entries {
        sqlx::query(
            "INSERT INTO channel_sender_keys (channel_id, sender_id, recipient_id, ciphertext) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (channel_id, sender_id, recipient_id) \
             DO UPDATE SET ciphertext = EXCLUDED.ciphertext, created_at = now()",
        )
        .bind(channel_id)
        .bind(session.user_id)
        .bind(entry.recipient_id)
        .bind(&entry.ciphertext)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SenderKeyDto {
    pub sender_id: Uuid,
    pub sender_username: String,
    pub ciphertext: String,
}

pub async fn list_sender_keys(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<SenderKeyDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let rows: Vec<SenderKeyDto> = sqlx::query_as(
        "SELECT channel_sender_keys.sender_id, users.username AS sender_username, \
                channel_sender_keys.ciphertext \
         FROM channel_sender_keys \
         JOIN users ON users.id = channel_sender_keys.sender_id \
         WHERE channel_sender_keys.channel_id = $1 AND channel_sender_keys.recipient_id = $2",
    )
    .bind(channel_id)
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}
