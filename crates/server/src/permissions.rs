use uuid::Uuid;

use crate::error::AppError;

pub const MANAGE_CHANNELS: i64 = 1 << 0;
pub const MANAGE_ROLES: i64 = 1 << 1;
pub const MANAGE_SERVER: i64 = 1 << 2;
pub const KICK_MEMBERS: i64 = 1 << 3;
pub const BAN_MEMBERS: i64 = 1 << 4;
pub const MANAGE_MESSAGES: i64 = 1 << 5;
pub const CREATE_INVITE: i64 = 1 << 6;

pub const ALL_PERMISSIONS: i64 = MANAGE_CHANNELS
    | MANAGE_ROLES
    | MANAGE_SERVER
    | KICK_MEMBERS
    | BAN_MEMBERS
    | MANAGE_MESSAGES
    | CREATE_INVITE;

pub async fn effective_permissions(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    user_id: Uuid,
) -> Result<i64, AppError> {
    let owner: Option<(Uuid,)> = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    let (owner_id,) = owner.ok_or(AppError::NotFound)?;
    if owner_id == user_id {
        return Ok(ALL_PERMISSIONS);
    }

    let rows: Vec<(i64,)> = sqlx::query_as(
        "SELECT server_roles.permissions FROM server_member_roles \
         JOIN server_roles ON server_roles.id = server_member_roles.role_id \
         WHERE server_member_roles.server_id = $1 AND server_member_roles.user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().fold(0, |acc, (p,)| acc | p))
}

pub async fn require_permission(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    user_id: Uuid,
    required: i64,
) -> Result<(), AppError> {
    let perms = effective_permissions(pool, server_id, user_id).await?;
    if perms & required == required {
        Ok(())
    } else {
        Err(AppError::Unauthorized)
    }
}
