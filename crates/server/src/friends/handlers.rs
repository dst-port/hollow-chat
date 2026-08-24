use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::social::{are_blocked, are_friends, ordered_pair, user_id_by_username};
use crate::state::AppState;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct FriendDto {
    pub id: Uuid,
    pub username: String,
}

pub async fn list_friends(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<FriendDto>>, AppError> {
    let friends: Vec<FriendDto> = sqlx::query_as(
        "SELECT users.id, users.username FROM friendships \
         JOIN users ON users.id = CASE WHEN friendships.user_a = $1 THEN friendships.user_b ELSE friendships.user_a END \
         WHERE friendships.user_a = $1 OR friendships.user_b = $1 \
         ORDER BY users.username",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(friends))
}

pub async fn remove_friend(
    State(state): State<AppState>,
    session: AuthSession,
    Path(user_id): Path<Uuid>,
) -> Result<(), AppError> {
    let (user_a, user_b) = ordered_pair(session.user_id, user_id);
    sqlx::query("DELETE FROM friendships WHERE user_a = $1 AND user_b = $2")
        .bind(user_a)
        .bind(user_b)
        .execute(&state.pool)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RequestDto {
    pub id: Uuid,
    pub username: String,
    pub direction: String,
}

pub async fn list_requests(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<RequestDto>>, AppError> {
    let requests: Vec<RequestDto> = sqlx::query_as(
        "SELECT friend_requests.id, \
                users.username, \
                CASE WHEN friend_requests.recipient_id = $1 THEN 'incoming' ELSE 'outgoing' END AS direction \
         FROM friend_requests \
         JOIN users ON users.id = CASE WHEN friend_requests.recipient_id = $1 \
                                        THEN friend_requests.sender_id \
                                        ELSE friend_requests.recipient_id END \
         WHERE friend_requests.sender_id = $1 OR friend_requests.recipient_id = $1 \
         ORDER BY friend_requests.created_at DESC",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(requests))
}

#[derive(Debug, Deserialize)]
pub struct SendRequestBody {
    pub username: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "result")]
pub enum SendRequestResponse {
    #[serde(rename = "sent")]
    Sent { id: Uuid },
    #[serde(rename = "accepted")]
    Accepted,
}

pub async fn send_request(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<SendRequestBody>,
) -> Result<Json<SendRequestResponse>, AppError> {
    let target_id = user_id_by_username(&state.pool, payload.username.trim()).await?;

    if target_id == session.user_id {
        return Err(AppError::InvalidName);
    }

    if are_friends(&state.pool, session.user_id, target_id).await? {
        return Err(AppError::AlreadyFriends);
    }
    if are_blocked(&state.pool, session.user_id, target_id).await? {
        return Err(AppError::Blocked);
    }

    let reverse: Option<(Uuid,)> = sqlx::query_as(
        "SELECT id FROM friend_requests WHERE sender_id = $1 AND recipient_id = $2",
    )
    .bind(target_id)
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await?;

    if let Some((request_id,)) = reverse {
        let (user_a, user_b) = ordered_pair(session.user_id, target_id);
        let mut tx = state.pool.begin().await?;
        sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2)")
            .bind(user_a)
            .bind(user_b)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM friend_requests WHERE id = $1")
            .bind(request_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        return Ok(Json(SendRequestResponse::Accepted));
    }

    let result = sqlx::query(
        "INSERT INTO friend_requests (sender_id, recipient_id) VALUES ($1, $2) RETURNING id",
    )
    .bind(session.user_id)
    .bind(target_id)
    .fetch_one(&state.pool)
    .await;

    let row = match result {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            return Err(AppError::RequestAlreadySent);
        }
        Err(err) => return Err(err.into()),
    };

    let id: Uuid = sqlx::Row::try_get(&row, "id")?;

    Ok(Json(SendRequestResponse::Sent { id }))
}

async fn load_request(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Result<(Uuid, Uuid), AppError> {
    let row: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT sender_id, recipient_id FROM friend_requests WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await?;
    row.ok_or(AppError::NotFound)
}

pub async fn accept_request(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let (sender_id, recipient_id) = load_request(&state.pool, id).await?;
    if recipient_id != session.user_id {
        return Err(AppError::Unauthorized);
    }

    let (user_a, user_b) = ordered_pair(sender_id, recipient_id);
    let mut tx = state.pool.begin().await?;
    sqlx::query("INSERT INTO friendships (user_a, user_b) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_a)
        .bind(user_b)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM friend_requests WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    Ok(())
}

pub async fn decline_request(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let (sender_id, recipient_id) = load_request(&state.pool, id).await?;
    if sender_id != session.user_id && recipient_id != session.user_id {
        return Err(AppError::Unauthorized);
    }

    sqlx::query("DELETE FROM friend_requests WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(())
}
