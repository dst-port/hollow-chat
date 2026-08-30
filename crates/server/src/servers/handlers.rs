use axum::extract::{Path, State};
use axum::Json;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::gateway::{notify_sync, server_member_ids};
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

fn file_url(attachment_id: Uuid, filename: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    format!(
        "/files/{attachment_id}/{}",
        utf8_percent_encode(filename, NON_ALPHANUMERIC)
    )
}

async fn owned_image_attachment(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    attachment_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(String,)> =
        sqlx::query_as("SELECT mime_type FROM attachments WHERE id = $1 AND uploader_id = $2")
            .bind(attachment_id)
            .bind(user_id)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((mime,)) if mime.starts_with("image/") => Ok(()),
        _ => Err(AppError::InvalidAttachment),
    }
}

async fn load_server_dto(pool: &sqlx::PgPool, server_id: Uuid) -> Result<ServerDto, AppError> {
    let row: ServerRow = sqlx::query_as(
        "SELECT servers.id, servers.name, servers.owner_id, \
                servers.icon_attachment_id, icon.filename AS icon_filename, \
                (SELECT count(*) FROM server_boosts WHERE server_boosts.server_id = servers.id) AS boost_count \
         FROM servers \
         LEFT JOIN attachments icon ON icon.id = servers.icon_attachment_id \
         WHERE servers.id = $1",
    )
    .bind(server_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(row.into())
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

#[derive(Debug, Serialize)]
pub struct ServerDto {
    pub id: Uuid,
    pub name: String,
    pub owner_id: Uuid,
    pub icon_url: Option<String>,
    pub boost_count: i64,
}

#[derive(sqlx::FromRow)]
struct ServerRow {
    id: Uuid,
    name: String,
    owner_id: Uuid,
    icon_attachment_id: Option<Uuid>,
    icon_filename: Option<String>,
    boost_count: i64,
}

impl From<ServerRow> for ServerDto {
    fn from(row: ServerRow) -> Self {
        ServerDto {
            id: row.id,
            name: row.name,
            owner_id: row.owner_id,
            icon_url: row
                .icon_attachment_id
                .zip(row.icon_filename)
                .map(|(id, name)| file_url(id, &name)),
            boost_count: row.boost_count,
        }
    }
}

/// Custom-emoji slots a server gets from its total Void Shards - the boost
/// count is small at this scale (friend-group servers, not thousands of
/// boosters), so the tiers stay simple: a modest free allowance, then a
/// jump at 1 boost and another at 3.
pub fn emoji_slots_for_boosts(boost_count: i64) -> i64 {
    match boost_count {
        0 => 5,
        1..=2 => 15,
        _ => 30,
    }
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

    let (id, saved_name, owner_id): (Uuid, String, Uuid) = sqlx::query_as(
        "INSERT INTO servers (name, owner_id) VALUES ($1, $2) RETURNING id, name, owner_id",
    )
    .bind(&name)
    .bind(session.user_id)
    .fetch_one(&mut *tx)
    .await?;
    let server = ServerDto { id, name: saved_name, owner_id, icon_url: None, boost_count: 0 };

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

    // Creator's other devices pick up the new server.
    notify_sync(&state, &[session.user_id], "servers");

    Ok(Json(ServerWithChannels { server, channels }))
}

pub async fn list_servers(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<ServerWithChannels>>, AppError> {
    let rows: Vec<ServerRow> = sqlx::query_as(
        "SELECT servers.id, servers.name, servers.owner_id, \
                servers.icon_attachment_id, icon.filename AS icon_filename, \
                (SELECT count(*) FROM server_boosts WHERE server_boosts.server_id = servers.id) AS boost_count \
         FROM servers \
         JOIN server_members ON server_members.server_id = servers.id \
         LEFT JOIN attachments icon ON icon.id = servers.icon_attachment_id \
         WHERE server_members.user_id = $1 \
         ORDER BY servers.created_at",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut result = Vec::with_capacity(rows.len());
    for row in rows {
        let server: ServerDto = row.into();
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

    sqlx::query("UPDATE servers SET name = $1 WHERE id = $2")
        .bind(&name)
        .bind(id)
        .execute(&state.pool)
        .await?;

    let server = load_server_dto(&state.pool, id).await?;

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

    Ok(Json(server))
}

#[derive(Debug, Deserialize)]
pub struct SetServerIconRequest {
    pub attachment_id: Uuid,
}

pub async fn set_server_icon(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
    Json(payload): Json<SetServerIconRequest>,
) -> Result<Json<ServerDto>, AppError> {
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;
    owned_image_attachment(&state.pool, session.user_id, payload.attachment_id).await?;

    sqlx::query("UPDATE servers SET icon_attachment_id = $1 WHERE id = $2")
        .bind(payload.attachment_id)
        .bind(id)
        .execute(&state.pool)
        .await?;

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

    Ok(Json(load_server_dto(&state.pool, id).await?))
}

pub async fn clear_server_icon(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<ServerDto>, AppError> {
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;

    sqlx::query("UPDATE servers SET icon_attachment_id = NULL WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

    Ok(Json(load_server_dto(&state.pool, id).await?))
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

    // Everyone who can currently see this server in their list - captured
    // before the delete so a full server teardown still reaches all of them.
    let affected = server_member_ids(&state.pool, id).await;

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

    notify_sync(&state, &affected, "servers");

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

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

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

    notify_sync(&state, &server_member_ids(&state.pool, server_id).await, "servers");

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

    let mut affected = server_member_ids(&state.pool, server_id).await;
    if !affected.contains(&user_id) {
        affected.push(user_id);
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

    notify_sync(&state, &affected, "servers");

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

    let mut affected = server_member_ids(&state.pool, server_id).await;
    if !affected.contains(&user_id) {
        affected.push(user_id);
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

    notify_sync(&state, &affected, "servers");

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

fn validate_vanity_code(code: &str) -> Result<String, AppError> {
    let trimmed = code.trim();
    let len = trimmed.chars().count();
    if !(3..=32).contains(&len) {
        return Err(AppError::InvalidInviteCode);
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
        return Err(AppError::InvalidInviteCode);
    }
    Ok(trimmed.to_string())
}

#[derive(Debug, Deserialize)]
pub struct SetVanityInviteRequest {
    pub code: String,
}

/// Premium perk: a server owner can pick their own invite code instead of
/// the random one, e.g. hollowchat.org/invite/monkes instead of a 10-char
/// string. Tied to the *owner's* subscription, not the server's boost
/// level - boosts are a shared, revocable perk from whoever assigned them,
/// this is closer to a personal vanity purchase.
pub async fn set_vanity_invite(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
    Json(payload): Json<SetVanityInviteRequest>,
) -> Result<Json<InviteDto>, AppError> {
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;
    if !crate::billing::is_premium(&state.pool, session.user_id).await? {
        return Err(AppError::NotPremium);
    }

    let code = validate_vanity_code(&payload.code)?;

    let result = sqlx::query(
        "INSERT INTO server_invites (server_id, code) VALUES ($1, $2) \
         ON CONFLICT (server_id) DO UPDATE SET code = excluded.code",
    )
    .bind(id)
    .bind(&code)
    .execute(&state.pool)
    .await;

    match result {
        Ok(_) => Ok(Json(InviteDto { code })),
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            Err(AppError::InvalidInviteCode)
        }
        Err(err) => Err(err.into()),
    }
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

    let server = load_server_dto(&state.pool, server_id).await?;

    let channels = load_channels(&state.pool, server.id).await?;

    // Joiner's other devices + everyone already in the server (member list).
    notify_sync(&state, &server_member_ids(&state.pool, server_id).await, "servers");

    Ok(Json(ServerWithChannels { server, channels }))
}

#[derive(Debug, Serialize)]
pub struct BoostStatusDto {
    pub boost_count: i64,
    pub boosted_by_me: bool,
    /// How many of this user's boost slots are on THIS server (0..=slots_total).
    pub my_boost_count: i64,
    /// The user's total premium boost slots (0 for non-premium).
    pub slots_total: i64,
    pub emoji_slots: i64,
}

pub async fn get_boosts(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<BoostStatusDto>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    let (boost_count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM server_boosts WHERE server_id = $1")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    let (my_boost_count,): (i64,) = sqlx::query_as(
        "SELECT count(*) FROM server_boosts WHERE server_id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await?;

    let slots_total = if crate::billing::is_premium(&state.pool, session.user_id).await? {
        crate::billing::PREMIUM_BOOST_SLOTS
    } else {
        0
    };

    Ok(Json(BoostStatusDto {
        boost_count,
        boosted_by_me: my_boost_count > 0,
        my_boost_count,
        slots_total,
        emoji_slots: emoji_slots_for_boosts(boost_count),
    }))
}

pub async fn add_boost(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<BoostStatusDto>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    if !crate::billing::is_premium(&state.pool, session.user_id).await? {
        return Err(AppError::NotPremium);
    }

    let (used,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM server_boosts WHERE user_id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await?;
    if used >= crate::billing::PREMIUM_BOOST_SLOTS {
        return Err(AppError::BoostSlotsFull);
    }

    // One row per boost; a user may stack multiple on the same server up to
    // their total slot count (checked above).
    sqlx::query("INSERT INTO server_boosts (user_id, server_id) VALUES ($1, $2)")
        .bind(session.user_id)
        .bind(id)
        .execute(&state.pool)
        .await?;

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

    get_boosts(State(state), session, Path(id)).await
}

pub async fn remove_boost(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<BoostStatusDto>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    // Remove a single boost (the most recent), not every boost this user has
    // on the server - they may have stacked more than one.
    sqlx::query(
        "DELETE FROM server_boosts WHERE id = ( \
             SELECT id FROM server_boosts \
             WHERE user_id = $1 AND server_id = $2 \
             ORDER BY id DESC LIMIT 1)",
    )
    .bind(session.user_id)
    .bind(id)
    .execute(&state.pool)
    .await?;

    notify_sync(&state, &server_member_ids(&state.pool, id).await, "servers");

    get_boosts(State(state), session, Path(id)).await
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CustomEmojiDto {
    pub id: Uuid,
    pub name: String,
    pub image_url: String,
}

#[derive(sqlx::FromRow)]
struct CustomEmojiRow {
    id: Uuid,
    name: String,
    attachment_id: Uuid,
    filename: String,
}

impl From<CustomEmojiRow> for CustomEmojiDto {
    fn from(row: CustomEmojiRow) -> Self {
        CustomEmojiDto {
            id: row.id,
            name: row.name,
            image_url: file_url(row.attachment_id, &row.filename),
        }
    }
}

pub async fn list_custom_emoji(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<CustomEmojiDto>>, AppError> {
    require_member(&state.pool, id, session.user_id).await?;

    let rows: Vec<CustomEmojiRow> = sqlx::query_as(
        "SELECT custom_emoji.id, custom_emoji.name, custom_emoji.attachment_id, \
                attachments.filename \
         FROM custom_emoji \
         JOIN attachments ON attachments.id = custom_emoji.attachment_id \
         WHERE custom_emoji.server_id = $1 \
         ORDER BY custom_emoji.created_at",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows.into_iter().map(CustomEmojiDto::from).collect()))
}

fn validate_emoji_name(name: &str) -> Result<String, AppError> {
    let trimmed = name.trim().trim_matches(':');
    let len = trimmed.chars().count();
    if !(2..=32).contains(&len) {
        return Err(AppError::InvalidEmojiName);
    }
    if !trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(AppError::InvalidEmojiName);
    }
    Ok(trimmed.to_lowercase())
}

#[derive(Debug, Deserialize)]
pub struct AddCustomEmojiRequest {
    pub name: String,
    pub attachment_id: Uuid,
}

pub async fn add_custom_emoji(
    State(state): State<AppState>,
    session: AuthSession,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddCustomEmojiRequest>,
) -> Result<Json<CustomEmojiDto>, AppError> {
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;
    owned_image_attachment(&state.pool, session.user_id, payload.attachment_id).await?;

    let name = validate_emoji_name(&payload.name)?;

    let (boost_count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM server_boosts WHERE server_id = $1")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;
    let (used,): (i64,) = sqlx::query_as("SELECT count(*) FROM custom_emoji WHERE server_id = $1")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;
    if used >= emoji_slots_for_boosts(boost_count) {
        return Err(AppError::EmojiSlotsFull);
    }

    let inserted: Result<(Uuid,), sqlx::Error> = sqlx::query_as(
        "INSERT INTO custom_emoji (server_id, name, attachment_id, created_by) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(id)
    .bind(&name)
    .bind(payload.attachment_id)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await;

    let (emoji_id,) = match inserted {
        Ok(row) => row,
        Err(sqlx::Error::Database(db_err)) if db_err.is_unique_violation() => {
            return Err(AppError::EmojiNameTaken)
        }
        Err(err) => return Err(err.into()),
    };

    let row: CustomEmojiRow = sqlx::query_as(
        "SELECT custom_emoji.id, custom_emoji.name, custom_emoji.attachment_id, \
                attachments.filename \
         FROM custom_emoji \
         JOIN attachments ON attachments.id = custom_emoji.attachment_id \
         WHERE custom_emoji.id = $1",
    )
    .bind(emoji_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row.into()))
}

pub async fn remove_custom_emoji(
    State(state): State<AppState>,
    session: AuthSession,
    Path((id, emoji_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_permission(&state.pool, id, session.user_id, MANAGE_SERVER).await?;

    sqlx::query("DELETE FROM custom_emoji WHERE id = $1 AND server_id = $2")
        .bind(emoji_id)
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(())
}
