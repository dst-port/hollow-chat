use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::permissions::{require_permission, ALL_PERMISSIONS, MANAGE_ROLES};
use crate::state::AppState;

fn validate_role_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if (1..=32).contains(&trimmed.chars().count()) {
        Ok(trimmed.to_string())
    } else {
        Err(AppError::InvalidName)
    }
}

async fn require_member(pool: &sqlx::PgPool, server_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    exists.map(|_| ()).ok_or(AppError::Unauthorized)
}

#[derive(Debug, Serialize, sqlx::FromRow, Clone)]
pub struct RoleDto {
    pub id: Uuid,
    pub server_id: Uuid,
    pub name: String,
    pub color: String,
    pub permissions: i64,
    pub position: i32,
}

const SELECT_ROLE: &str =
    "SELECT id, server_id, name, color, permissions, position FROM server_roles";

pub async fn load_member_roles(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    user_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<RoleDto>>, AppError> {
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        user_id: Uuid,
        id: Uuid,
        server_id: Uuid,
        name: String,
        color: String,
        permissions: i64,
        position: i32,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT server_member_roles.user_id, server_roles.id, server_roles.server_id, \
                server_roles.name, server_roles.color, server_roles.permissions, server_roles.position \
         FROM server_member_roles \
         JOIN server_roles ON server_roles.id = server_member_roles.role_id \
         WHERE server_member_roles.server_id = $1 AND server_member_roles.user_id = ANY($2) \
         ORDER BY server_roles.position DESC",
    )
    .bind(server_id)
    .bind(user_ids)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<RoleDto>> = HashMap::new();
    for row in rows {
        map.entry(row.user_id).or_default().push(RoleDto {
            id: row.id,
            server_id: row.server_id,
            name: row.name,
            color: row.color,
            permissions: row.permissions,
            position: row.position,
        });
    }
    Ok(map)
}

pub async fn list_roles(
    State(state): State<AppState>,
    session: AuthSession,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<RoleDto>>, AppError> {
    require_member(&state.pool, server_id, session.user_id).await?;

    let roles: Vec<RoleDto> = sqlx::query_as(&format!(
        "{SELECT_ROLE} WHERE server_id = $1 ORDER BY position DESC, created_at ASC"
    ))
    .bind(server_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(roles))
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    #[serde(default = "default_role_color")]
    pub color: String,
    #[serde(default)]
    pub permissions: i64,
}

fn default_role_color() -> String {
    "#8a8f98".to_string()
}

pub async fn create_role(
    State(state): State<AppState>,
    session: AuthSession,
    Path(server_id): Path<Uuid>,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<Json<RoleDto>, AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_ROLES).await?;

    let name = validate_role_name(&payload.name)?;
    let permissions = payload.permissions & ALL_PERMISSIONS;

    let position: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM server_roles WHERE server_id = $1")
        .bind(server_id)
        .fetch_one(&state.pool)
        .await?;

    let role: RoleDto = sqlx::query_as(&format!(
        "WITH inserted AS ( \
            INSERT INTO server_roles (server_id, name, color, permissions, position) \
            VALUES ($1, $2, $3, $4, $5) RETURNING * \
         ) SELECT id, server_id, name, color, permissions, position FROM inserted"
    ))
    .bind(server_id)
    .bind(&name)
    .bind(&payload.color)
    .bind(permissions)
    .bind(position.0 as i32)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(role))
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub color: Option<String>,
    pub permissions: Option<i64>,
    pub position: Option<i32>,
}

async fn require_same_server_role(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT server_id FROM server_roles WHERE id = $1")
        .bind(role_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((sid,)) if sid == server_id => Ok(()),
        Some(_) => Err(AppError::NotFound),
        None => Err(AppError::NotFound),
    }
}

pub async fn update_role(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, role_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<RoleDto>, AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_ROLES).await?;
    require_same_server_role(&state.pool, server_id, role_id).await?;

    let name = match payload.name {
        Some(n) => Some(validate_role_name(&n)?),
        None => None,
    };
    let permissions = payload.permissions.map(|p| p & ALL_PERMISSIONS);

    let role: RoleDto = sqlx::query_as(
        "UPDATE server_roles SET \
            name = COALESCE($1, name), \
            color = COALESCE($2, color), \
            permissions = COALESCE($3, permissions), \
            position = COALESCE($4, position) \
         WHERE id = $5 \
         RETURNING id, server_id, name, color, permissions, position",
    )
    .bind(name)
    .bind(payload.color)
    .bind(permissions)
    .bind(payload.position)
    .bind(role_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(role))
}

pub async fn delete_role(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, role_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_ROLES).await?;
    require_same_server_role(&state.pool, server_id, role_id).await?;

    sqlx::query("DELETE FROM server_roles WHERE id = $1")
        .bind(role_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}

pub async fn assign_role(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, user_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_ROLES).await?;
    require_same_server_role(&state.pool, server_id, role_id).await?;
    require_member(&state.pool, server_id, user_id).await?;

    sqlx::query(
        "INSERT INTO server_member_roles (server_id, user_id, role_id) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(role_id)
    .execute(&state.pool)
    .await?;

    Ok(())
}

pub async fn unassign_role(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, user_id, role_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_ROLES).await?;

    sqlx::query(
        "DELETE FROM server_member_roles WHERE server_id = $1 AND user_id = $2 AND role_id = $3",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(role_id)
    .execute(&state.pool)
    .await?;

    Ok(())
}
