use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::permissions::{require_permission, BAN_MEMBERS, KICK_MEMBERS, MANAGE_CHANNELS, MANAGE_SERVER};
use crate::roles::handlers::{load_member_roles, RoleDto};
use crate::state::AppState;

fn validate_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim();
    if (1..=48).contains(&trimmed.chars().count()) {
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChannelDto {
    pub id: Uuid,
    pub name: String,
    #[sqlx(rename = "channel_type")]
    #[serde(rename = "type")]
    pub kind: String,
    pub category: Option<String>,
    pub slowmode_seconds: i32,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ServerDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct ServerWithChannels {
    #[serde(flatten)]
    pub server: ServerDto,
    pub channels: Vec<ChannelDto>,
}

async fn load_channels(pool: &sqlx::PgPool, server_id: Uuid) -> Result<Vec<ChannelDto>, AppError> {
    let channels = sqlx::query_as::<_, ChannelDto>(
        "SELECT id, name, channel_type, category, slowmode_seconds FROM channels \
         WHERE server_id = $1 ORDER BY position, created_at",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;
    Ok(channels)
}

#[derive(Debug, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
}

pub async fn create_server(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<CreateServerRequest>,
) -> Result<Json<ServerWithChannels>, AppError> {
    let name = validate_name(&payload.name)?;

    let mut tx = state.pool.begin().await?;

    let server: ServerDto = sqlx::query_as(
        "INSERT INTO servers (name, owner_id) VALUES ($1, $2) RETURNING id, name, owner_id",
    )
    .bind(&name)
    .bind(session.user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query("INSERT INTO server_members (server_id, user_id) VALUES ($1, $2)")
        .bind(server.id)
        .bind(session.user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO channels (name, server_id, channel_type, position) VALUES ($1, $2, 'text', 0)",
    )
    .bind("general")
    .bind(server.id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO channels (name, server_id, channel_type, position) VALUES ($1, $2, 'voice', 1)",
    )
    .bind("General Voice")
    .bind(server.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    let channels = load_channels(&state.pool, server.id).await?;

    Ok(Json(ServerWithChannels { server, channels }))
}

pub async fn list_servers(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<ServerWithChannels>>, AppError> {
    let servers: Vec<ServerDto> = sqlx::query_as(
        "SELECT servers.id, servers.name, servers.owner_id FROM servers \
         JOIN server_members ON server_members.server_id = servers.id \
         WHERE server_members.user_id = $1 \
         ORDER BY servers.created_at",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::with_capacity(servers.len());
    for server in servers {
        let channels = load_channels(&state.pool, server.id).await?;
        result.push(ServerWithChannels { server, channels });
    }

    Ok(Json(result))
}

#[derive(Debug, Deserialize)]
pub struct RenameServerRequest {
    pub name: String,
}

pub async fn rename_server(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
    Json(payload): Json<RenameServerRequest>,
) -> Result<Json<ServerDto>, AppError> {
    let name = validate_name(&payload.name)?;
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;

    let server: ServerDto = sqlx::query_as(
        "UPDATE servers SET name = $1 WHERE id = $2 RETURNING id, name, owner_id",
    )
    .bind(&name)
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(server))
}

pub async fn leave_server(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    let owner: Option<(Uuid,)> = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?;

    match owner {
        Some((owner_id,)) if owner_id == session.user_id => {
            sqlx::query("DELETE FROM servers WHERE id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?;
        }
        Some(_) => {
            sqlx::query("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2")
                .bind(id)
                .bind(session.user_id)
                .execute(&state.pool)
                .await?;
        }
        None => return Err(AppError::NotFound),
    }

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct CreateChannelRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub category: Option<String>,
}

pub async fn create_channel(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateChannelRequest>,
) -> Result<Json<ChannelDto>, AppError> {
    let name = validate_name(&payload.name)?;
    if payload.kind != "text" && payload.kind != "voice" {
        return Err(AppError::InvalidChannelType);
    }
    require_permission(&state.pool, id, session.user_id, MANAGE_CHANNELS).await?;

    let position: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM channels WHERE server_id = $1",
    )
    .bind(id)
    .fetch_one(&state.pool)
    .await?;

    let channel: ChannelDto = sqlx::query_as(
        "INSERT INTO channels (name, server_id, channel_type, category, position) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, name, channel_type, category, slowmode_seconds",
    )
    .bind(&name)
    .bind(id)
    .bind(&payload.kind)
    .bind(&payload.category)
    .bind(position.0 as i32)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(channel))
}

#[derive(Debug, Deserialize)]
pub struct SetSlowmodeRequest {
    pub seconds: i32,
}

pub async fn set_slowmode(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, channel_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SetSlowmodeRequest>,
) -> Result<Json<ChannelDto>, AppError> {
    require_permission(&state.pool, server_id, session.user_id, MANAGE_CHANNELS).await?;
    let seconds = payload.seconds.clamp(0, 21600);

    let channel: ChannelDto = sqlx::query_as(
        "UPDATE channels SET slowmode_seconds = $1 WHERE id = $2 AND server_id = $3 \
         RETURNING id, name, channel_type, category, slowmode_seconds",
    )
    .bind(seconds)
    .bind(channel_id)
    .bind(server_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(channel))
}

#[derive(Debug, Serialize)]
pub struct MemberDto {
    pub id: Uuid,
    pub username: String,
    pub is_owner: bool,
    pub roles: Vec<RoleDto>,
}

pub async fn list_members(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MemberDto>>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        username: String,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT users.id, users.username FROM server_members \
         JOIN users ON users.id = server_members.user_id \
         WHERE server_members.server_id = $1 \
         ORDER BY users.username",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let owner: (Uuid,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    let user_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let mut roles_by_user = load_member_roles(&state.pool, id, &user_ids).await?;

    let members = rows
        .into_iter()
        .map(|r| MemberDto {
            is_owner: r.id == owner.0,
            roles: roles_by_user.remove(&r.id).unwrap_or_default(),
            id: r.id,
            username: r.username,
        })
        .collect();

    Ok(Json(members))
}

pub async fn kick_member(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, KICK_MEMBERS).await?;

    let owner: (Uuid,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(&state.pool)
        .await?;
    if owner.0 == user_id {
        return Err(AppError::Unauthorized);
    }

    let mut tx = state.pool.begin().await?;

    sqlx::query("DELETE FROM server_member_roles WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BanDto {
    pub user_id: Uuid,
    pub username: String,
    pub reason: Option<String>,
}

pub async fn list_bans(
    State(state): State<AppState>,
    session: AuthSession,
    Path(server_id): Path<Uuid>,
) -> Result<Json<Vec<BanDto>>, AppError> {
    require_permission(&state.pool, server_id, session.user_id, BAN_MEMBERS).await?;

    let bans: Vec<BanDto> = sqlx::query_as(
        "SELECT server_bans.user_id, users.username, server_bans.reason FROM server_bans \
         JOIN users ON users.id = server_bans.user_id \
         WHERE server_bans.server_id = $1 \
         ORDER BY server_bans.created_at DESC",
    )
    .bind(server_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(bans))
}

#[derive(Debug, Deserialize)]
pub struct BanMemberRequest {
    #[serde(default)]
    pub reason: Option<String>,
}

pub async fn ban_member(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, user_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<BanMemberRequest>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, BAN_MEMBERS).await?;

    let owner: (Uuid,) = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(&state.pool)
        .await?;
    if owner.0 == user_id {
        return Err(AppError::Unauthorized);
    }

    let mut tx = state.pool.begin().await?;

    sqlx::query(
        "INSERT INTO server_bans (server_id, user_id, banned_by, reason) VALUES ($1, $2, $3, $4) \
         ON CONFLICT (server_id, user_id) DO UPDATE SET reason = EXCLUDED.reason",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(session.user_id)
    .bind(&payload.reason)
    .execute(&mut *tx)
    .await?;

    sqlx::query("DELETE FROM server_member_roles WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    sqlx::query("DELETE FROM server_members WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn unban_member(
    State(state): State<AppState>,
    session: AuthSession,
    Path((server_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, server_id, session.user_id, BAN_MEMBERS).await?;

    sqlx::query("DELETE FROM server_bans WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}

fn generate_invite_code() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    (0..10)
        .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
        .collect()
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct InviteDto {
    pub code: String,
}

pub async fn get_invite(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<InviteDto>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    let existing: Option<InviteDto> =
        sqlx::query_as("SELECT code FROM server_invites WHERE server_id = $1")
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    if let Some(invite) = existing {
        return Ok(Json(invite));
    }

    for _ in 0..5 {
        let code = generate_invite_code();
        let result = sqlx::query("INSERT INTO server_invites (server_id, code) VALUES ($1, $2)")
            .bind(id)
            .bind(&code)
            .execute(&state.pool)
            .await;

        match result {
            Ok(_) => return Ok(Json(InviteDto { code })),
            Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => continue,
            Err(err) => return Err(err.into()),
        }
    }

    Err(AppError::InviteGenerationFailed)
}

#[derive(Debug, Deserialize)]
pub struct JoinServerRequest {
    pub code: String,
}

pub async fn join_server(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<JoinServerRequest>,
) -> Result<Json<ServerWithChannels>, AppError> {
    let code = payload.code.trim();

    let server_id: Option<(Uuid,)> =
        sqlx::query_as("SELECT server_id FROM server_invites WHERE code = $1")
            .bind(code)
            .fetch_optional(&state.pool)
            .await?;

    let (server_id,) = server_id.ok_or(AppError::NotFound)?;

    let banned: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_bans WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(session.user_id)
    .fetch_optional(&state.pool)
    .await?;
    if banned.is_some() {
        return Err(AppError::Unauthorized);
    }

    sqlx::query(
        "INSERT INTO server_members (server_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(session.user_id)
    .execute(&state.pool)
    .await?;

    let server: ServerDto = sqlx::query_as("SELECT id, name, owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_one(&state.pool)
        .await?;

    let channels = load_channels(&state.pool, server.id).await?;

    Ok(Json(ServerWithChannels { server, channels }))
}
