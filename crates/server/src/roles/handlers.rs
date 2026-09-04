use axum::extract::{Path, State};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::permissions::{effective_permissions, require_permission, ALL_PERMISSIONS, MANAGE_ROLES};
use crate::state::AppState;

/// Nobody may hand out a power they don't hold themselves.
///
/// MANAGE_ROLES is the bit that decides who gets which bits, so without a
/// ceiling it is a back door to every other permission in the server: mint a
/// role carrying all of them, assign it to yourself, and a moderator is now an
/// admin. The server owner short-circuits to ALL_PERMISSIONS, so this never
/// constrains them.
async fn require_permission_ceiling(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    actor_id: Uuid,
    permissions: i64,
) -> Result<(), AppError> {
    let mine = effective_permissions(pool, server_id, actor_id).await?;
    if permissions & !mine != 0 {
        return Err(AppError::Unauthorized);
    }
    Ok(())
}

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
    require_permission_ceiling(&state.pool, server_id, session.user_id, permissions).await?;

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

/// Checks the role belongs to this server and returns the powers it currently
/// carries, so callers can hold it against their own ceiling.
async fn require_same_server_role(
    pool: &sqlx::PgPool,
    server_id: Uuid,
    role_id: Uuid,
) -> Result<i64, AppError> {
    let row: Option<(Uuid, i64)> =
        sqlx::query_as("SELECT server_id, permissions FROM server_roles WHERE id = $1")
            .bind(role_id)
            .fetch_optional(pool)
            .await?;
    match row {
        Some((sid, permissions)) if sid == server_id => Ok(permissions),
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
    let current = require_same_server_role(&state.pool, server_id, role_id).await?;
    // Editing a role you couldn't have created is the same escalation by
    // another route - so you must already hold everything it currently grants,
    // as well as everything you're asking it to grant.
    require_permission_ceiling(&state.pool, server_id, session.user_id, current).await?;

    let name = match payload.name {
        Some(n) => Some(validate_role_name(&n)?),
        None => None,
    };
    let permissions = payload.permissions.map(|p| p & ALL_PERMISSIONS);
    if let Some(permissions) = permissions {
        require_permission_ceiling(&state.pool, server_id, session.user_id, permissions).await?;
    }

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
    let current = require_same_server_role(&state.pool, server_id, role_id).await?;
    require_permission_ceiling(&state.pool, server_id, session.user_id, current).await?;

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
    let granted = require_same_server_role(&state.pool, server_id, role_id).await?;
    // Handing an existing powerful role to yourself is the shortest path of
    // all, so the ceiling applies here too.
    require_permission_ceiling(&state.pool, server_id, session.user_id, granted).await?;
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
    // Stripping a role you couldn't grant would let a moderator demote an
    // admin, so the same ceiling applies in reverse.
    let held = require_same_server_role(&state.pool, server_id, role_id).await?;
    require_permission_ceiling(&state.pool, server_id, session.user_id, held).await?;

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
