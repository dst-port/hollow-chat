use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
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

async fn require_owner(pool: &sqlx::PgPool, server_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let owner: Option<(Uuid,)> = sqlx::query_as("SELECT owner_id FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(pool)
        .await?;

    match owner {
        Some((owner_id,)) if owner_id == user_id => Ok(()),
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::NotFound),
    }
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
    require_owner(&state.pool, id, session.user_id).await?;

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
    require_member(&state.pool, id, session.user_id).await?;

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
    require_owner(&state.pool, server_id, session.user_id).await?;
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MemberDto {
    pub id: Uuid,
    pub username: String,
}

pub async fn list_members(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<MemberDto>>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    let members: Vec<MemberDto> = sqlx::query_as(
        "SELECT users.id, users.username FROM server_members \
         JOIN users ON users.id = server_members.user_id \
         WHERE server_members.server_id = $1 \
         ORDER BY users.username",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(members))
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
