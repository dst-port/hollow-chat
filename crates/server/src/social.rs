use uuid::Uuid;

use crate::error::AppError;

pub fn ordered_pair(a: Uuid, b: Uuid) -> (Uuid, Uuid) {
    if a < b {
        (a, b)
    } else {
        (b, a)
    }
}

pub async fn user_id_by_username(pool: &sqlx::PgPool, username: &str) -> Result<Uuid, AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM users WHERE username = $1")
        .bind(username)
        .fetch_optional(pool)
        .await?;
    Ok(row.ok_or(AppError::NotFound)?.0)
}

pub async fn are_friends(pool: &sqlx::PgPool, a: Uuid, b: Uuid) -> Result<bool, AppError> {
    let (user_a, user_b) = ordered_pair(a, b);
    let row: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM friendships WHERE user_a = $1 AND user_b = $2",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}
