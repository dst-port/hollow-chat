use axum::extract::{Path, State};
use axum::Json;
use serde::Serialize;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::gateway::notify_sync;
use crate::social::ordered_pair;
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BlockedUserDto {
    pub id: Uuid,
    pub username: String,
}

pub async fn list_blocked(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<BlockedUserDto>>, AppError> {
    let rows: Vec<BlockedUserDto> = sqlx::query_as(
        "SELECT users.id, users.username FROM blocked_users \
         JOIN users ON users.id = blocked_users.blocked_id \
         WHERE blocked_users.blocker_id = $1 \
         ORDER BY users.username",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

pub async fn block_user(
    State(state): State<AppState>,
    session: AuthSession,
    Path(user_id): Path<Uuid>,
) -> Result<(), AppError> {
    if user_id == session.user_id {
        return Err(AppError::InvalidName);
    }

    let (a, b) = ordered_pair(session.user_id, user_id);
    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "INSERT INTO blocked_users (blocker_id, blocked_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(session.user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM friendships WHERE user_a = $1 AND user_b = $2")
        .bind(a)
        .bind(b)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "DELETE FROM friend_requests \
         WHERE (sender_id = $1 AND recipient_id = $2) \
            OR (sender_id = $2 AND recipient_id = $1)",
    )
    .bind(session.user_id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    // Blocking tears down any friendship and pending requests both ways, and
    // hides DMs - both sides refetch.
    notify_sync(&state, &[session.user_id, user_id], "friends");
    notify_sync(&state, &[session.user_id, user_id], "dms");

    Ok(())
}

pub async fn unblock_user(
    State(state): State<AppState>,
    session: AuthSession,
    Path(user_id): Path<Uuid>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM blocked_users WHERE blocker_id = $1 AND blocked_id = $2")
        .bind(session.user_id)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}
