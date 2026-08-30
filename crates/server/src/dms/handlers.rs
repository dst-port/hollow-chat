use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::gateway::{dm_member_ids, notify_sync, push_to_users, GatewayEvent};
use crate::social::{are_blocked, are_friends, ordered_pair, user_id_by_username};
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;
const MAX_CONTENT_LEN: usize = 4000;
const MAX_EMOJI_LEN: usize = 32;

const MAX_GROUP_MEMBERS: usize = 25;
const MAX_GROUP_NAME_LEN: usize = 100;

async fn require_participant(
    pool: &sqlx::PgPool,
    dm_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let exists: Option<(Uuid,)> = sqlx::query_as(
        "SELECT dm_channel_id FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id = $2",
    )
    .bind(dm_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    match exists {
        Some(_) => Ok(()),
        None => {
            let channel_exists: Option<(Uuid,)> =
                sqlx::query_as("SELECT id FROM dm_channels WHERE id = $1")
                    .bind(dm_id)
                    .fetch_optional(pool)
                    .await?;
            match channel_exists {
                Some(_) => Err(AppError::Unauthorized),
                None => Err(AppError::NotFound),
            }
        }
    }
}

fn validate_emoji(emoji: &str) -> Result<(), AppError> {
    let len_ok = (1..=MAX_EMOJI_LEN).contains(&emoji.chars().count());
    let no_whitespace = !emoji.chars().any(|c| c.is_whitespace() || c.is_control());
    if len_ok && no_whitespace {
        Ok(())
    } else {
        Err(AppError::InvalidEmoji)
    }
}

#[derive(Debug, Serialize)]
pub struct DmMemberDto {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct DmChannelDto {
    pub id: Uuid,
    pub is_group: bool,
    pub name: Option<String>,
    pub owner_id: Option<Uuid>,
    // Present (and only meaningful) for 1:1 DMs, kept for backward compatibility.
    pub peer_id: Option<Uuid>,
    pub peer_username: Option<String>,
    pub members: Vec<DmMemberDto>,
}

#[derive(sqlx::FromRow)]
struct DmChannelRow {
    id: Uuid,
    is_group: bool,
    name: Option<String>,
    owner_id: Option<Uuid>,
}

#[derive(sqlx::FromRow)]
struct DmMemberRow {
    dm_channel_id: Uuid,
    id: Uuid,
    username: String,
}

async fn load_members(
    pool: &sqlx::PgPool,
    channel_ids: &[Uuid],
) -> Result<HashMap<Uuid, Vec<DmMemberDto>>, AppError> {
    if channel_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let rows: Vec<DmMemberRow> = sqlx::query_as(
        "SELECT dm_channel_members.dm_channel_id, users.id, users.username \
         FROM dm_channel_members \
         JOIN users ON users.id = dm_channel_members.user_id \
         WHERE dm_channel_members.dm_channel_id = ANY($1) \
         ORDER BY users.username ASC",
    )
    .bind(channel_ids)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<DmMemberDto>> = HashMap::new();
    for row in rows {
        map.entry(row.dm_channel_id)
            .or_default()
            .push(DmMemberDto { id: row.id, username: row.username });
    }
    Ok(map)
}

fn assemble_dm_dto(row: DmChannelRow, requester_id: Uuid, members: Vec<DmMemberDto>) -> DmChannelDto {
    let (peer_id, peer_username) = if row.is_group {
        (None, None)
    } else {
        match members.iter().find(|m| m.id != requester_id) {
            Some(peer) => (Some(peer.id), Some(peer.username.clone())),
            None => (None, None),
        }
    };

    DmChannelDto {
        id: row.id,
        is_group: row.is_group,
        name: row.name,
        owner_id: row.owner_id,
        peer_id,
        peer_username,
        members,
    }
}

pub async fn list_dms(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<DmChannelDto>>, AppError> {
    let rows: Vec<DmChannelRow> = sqlx::query_as(
        "SELECT dm_channels.id, dm_channels.is_group, dm_channels.name, dm_channels.owner_id \
         FROM dm_channels \
         JOIN dm_channel_members ON dm_channel_members.dm_channel_id = dm_channels.id \
         WHERE dm_channel_members.user_id = $1 \
         ORDER BY dm_channels.created_at DESC",
    )
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    let channel_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let mut members = load_members(&state.pool, &channel_ids).await?;

    let dms = rows
        .into_iter()
        .map(|row| {
            let members = members.remove(&row.id).unwrap_or_default();
            assemble_dm_dto(row, session.user_id, members)
        })
        .collect();

    Ok(Json(dms))
}

async fn load_dm_dto(pool: &sqlx::PgPool, dm_id: Uuid, requester_id: Uuid) -> Result<DmChannelDto, AppError> {
    let row: DmChannelRow = sqlx::query_as(
        "SELECT id, is_group, name, owner_id FROM dm_channels WHERE id = $1",
    )
    .bind(dm_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    let mut members = load_members(pool, &[dm_id]).await?;
    let members = members.remove(&dm_id).unwrap_or_default();
    Ok(assemble_dm_dto(row, requester_id, members))
}

#[derive(Debug, Deserialize)]
pub struct OpenDmRequest {
    pub username: String,
}

pub async fn open_dm(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<OpenDmRequest>,
) -> Result<Json<DmChannelDto>, AppError> {
    let peer_id = user_id_by_username(&state.pool, payload.username.trim()).await?;
    if peer_id == session.user_id {
        return Err(AppError::InvalidName);
    }
    if !are_friends(&state.pool, session.user_id, peer_id).await? {
        return Err(AppError::Unauthorized);
    }
    if are_blocked(&state.pool, session.user_id, peer_id).await? {
        return Err(AppError::Blocked);
    }

    let (user_a, user_b) = ordered_pair(session.user_id, peer_id);

    let dm_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO dm_channels (user_a, user_b) VALUES ($1, $2) \
         ON CONFLICT (user_a, user_b) WHERE is_group = false \
         DO UPDATE SET user_a = EXCLUDED.user_a \
         RETURNING id",
    )
    .bind(user_a)
    .bind(user_b)
    .fetch_one(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO dm_channel_members (dm_channel_id, user_id) VALUES ($1, $2), ($1, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(dm_id.0)
    .bind(user_a)
    .bind(user_b)
    .execute(&state.pool)
    .await?;

    // The peer's client shows the new conversation without a reload.
    notify_sync(&state, &[user_a, user_b], "dms");

    let dm = load_dm_dto(&state.pool, dm_id.0, session.user_id).await?;
    Ok(Json(dm))
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupRequest {
    pub usernames: Vec<String>,
    #[serde(default)]
    pub name: Option<String>,
}

pub async fn create_group(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<CreateGroupRequest>,
) -> Result<Json<DmChannelDto>, AppError> {
    let mut member_ids: Vec<Uuid> = Vec::new();
    for raw in &payload.usernames {
        let username = raw.trim();
        if username.is_empty() {
            continue;
        }
        let peer_id = user_id_by_username(&state.pool, username).await?;
        if peer_id == session.user_id {
            continue;
        }
        if !are_friends(&state.pool, session.user_id, peer_id).await? {
            return Err(AppError::Unauthorized);
        }
        if are_blocked(&state.pool, session.user_id, peer_id).await? {
            return Err(AppError::Blocked);
        }
        if !member_ids.contains(&peer_id) {
            member_ids.push(peer_id);
        }
    }

    if member_ids.len() < 2 {
        return Err(AppError::InvalidMessage);
    }
    if member_ids.len() + 1 > MAX_GROUP_MEMBERS {
        return Err(AppError::InvalidMessage);
    }

    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| n.chars().take(MAX_GROUP_NAME_LEN).collect::<String>());

    let mut tx = state.pool.begin().await?;

    let dm_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO dm_channels (is_group, name, owner_id) VALUES (true, $1, $2) RETURNING id",
    )
    .bind(&name)
    .bind(session.user_id)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO dm_channel_members (dm_channel_id, user_id) VALUES ($1, $2)",
    )
    .bind(dm_id.0)
    .bind(session.user_id)
    .execute(&mut *tx)
    .await?;

    for member_id in &member_ids {
        sqlx::query(
            "INSERT INTO dm_channel_members (dm_channel_id, user_id) VALUES ($1, $2)",
        )
        .bind(dm_id.0)
        .bind(member_id)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let mut everyone = member_ids.clone();
    everyone.push(session.user_id);
    notify_sync(&state, &everyone, "dms");

    let dm = load_dm_dto(&state.pool, dm_id.0, session.user_id).await?;
    Ok(Json(dm))
}

async fn require_group(pool: &sqlx::PgPool, dm_id: Uuid) -> Result<(), AppError> {
    let row: Option<(bool,)> = sqlx::query_as("SELECT is_group FROM dm_channels WHERE id = $1")
        .bind(dm_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((true,)) => Ok(()),
        Some((false,)) => Err(AppError::InvalidMessage),
        None => Err(AppError::NotFound),
    }
}

#[derive(Debug, Deserialize)]
pub struct AddMemberRequest {
    pub username: String,
}

pub async fn add_member(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    Json(payload): Json<AddMemberRequest>,
) -> Result<Json<DmChannelDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_group(&state.pool, dm_id).await?;

    let peer_id = user_id_by_username(&state.pool, payload.username.trim()).await?;
    if !are_friends(&state.pool, session.user_id, peer_id).await? {
        return Err(AppError::Unauthorized);
    }
    if are_blocked(&state.pool, session.user_id, peer_id).await? {
        return Err(AppError::Blocked);
    }

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM dm_channel_members WHERE dm_channel_id = $1")
            .bind(dm_id)
            .fetch_one(&state.pool)
            .await?;
    if count.0 as usize >= MAX_GROUP_MEMBERS {
        return Err(AppError::InvalidMessage);
    }

    sqlx::query(
        "INSERT INTO dm_channel_members (dm_channel_id, user_id) VALUES ($1, $2) \
         ON CONFLICT DO NOTHING",
    )
    .bind(dm_id)
    .bind(peer_id)
    .execute(&state.pool)
    .await?;

    notify_sync(&state, &dm_member_ids(&state.pool, dm_id).await, "dms");

    let dm = load_dm_dto(&state.pool, dm_id, session.user_id).await?;
    Ok(Json(dm))
}

pub async fn leave_group(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
) -> Result<(), AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_group(&state.pool, dm_id).await?;

    // Captured before the delete so the leaver's own other devices are told too.
    let affected = dm_member_ids(&state.pool, dm_id).await;

    sqlx::query("DELETE FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id = $2")
        .bind(dm_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    let remaining: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM dm_channel_members WHERE dm_channel_id = $1")
            .bind(dm_id)
            .fetch_one(&state.pool)
            .await?;

    if remaining.0 == 0 {
        sqlx::query("DELETE FROM dm_channels WHERE id = $1")
            .bind(dm_id)
            .execute(&state.pool)
            .await?;
    } else {
        let owner: (Option<Uuid>,) =
            sqlx::query_as("SELECT owner_id FROM dm_channels WHERE id = $1")
                .bind(dm_id)
                .fetch_one(&state.pool)
                .await?;
        if owner.0 == Some(session.user_id) {
            sqlx::query(
                "UPDATE dm_channels SET owner_id = ( \
                    SELECT user_id FROM dm_channel_members WHERE dm_channel_id = $1 \
                    ORDER BY added_at ASC LIMIT 1 \
                 ) WHERE id = $1",
            )
            .bind(dm_id)
            .execute(&state.pool)
            .await?;
        }
    }

    notify_sync(&state, &affected, "dms");

    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct RenameGroupRequest {
    pub name: Option<String>,
}

pub async fn rename_group(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    Json(payload): Json<RenameGroupRequest>,
) -> Result<Json<DmChannelDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_group(&state.pool, dm_id).await?;

    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(|n| n.chars().take(MAX_GROUP_NAME_LEN).collect::<String>());

    sqlx::query("UPDATE dm_channels SET name = $1 WHERE id = $2")
        .bind(&name)
        .bind(dm_id)
        .execute(&state.pool)
        .await?;

    notify_sync(&state, &dm_member_ids(&state.pool, dm_id).await, "dms");

    let dm = load_dm_dto(&state.pool, dm_id, session.user_id).await?;
    Ok(Json(dm))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct DmSenderKeyDto {
    pub sender_id: Uuid,
    pub sender_username: String,
    pub ciphertext: String,
}

#[derive(Debug, Deserialize)]
pub struct DmSenderKeyEntry {
    pub recipient_id: Uuid,
    pub ciphertext: String,
}

#[derive(Debug, Deserialize)]
pub struct PublishDmSenderKeysRequest {
    pub entries: Vec<DmSenderKeyEntry>,
}

pub async fn publish_sender_keys(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    Json(payload): Json<PublishDmSenderKeysRequest>,
) -> Result<(), AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    if payload.entries.len() > MAX_GROUP_MEMBERS {
        return Err(AppError::InvalidMessage);
    }

    let mut tx = state.pool.begin().await?;
    for entry in &payload.entries {
        sqlx::query(
            "INSERT INTO dm_channel_sender_keys (dm_channel_id, sender_id, recipient_id, ciphertext) \
             VALUES ($1, $2, $3, $4) \
             ON CONFLICT (dm_channel_id, sender_id, recipient_id) \
             DO UPDATE SET ciphertext = EXCLUDED.ciphertext, created_at = now()",
        )
        .bind(dm_id)
        .bind(session.user_id)
        .bind(entry.recipient_id)
        .bind(&entry.ciphertext)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}

pub async fn list_sender_keys(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
) -> Result<Json<Vec<DmSenderKeyDto>>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    let rows: Vec<DmSenderKeyDto> = sqlx::query_as(
        "SELECT dm_channel_sender_keys.sender_id, users.username AS sender_username, \
                dm_channel_sender_keys.ciphertext \
         FROM dm_channel_sender_keys \
         JOIN users ON users.id = dm_channel_sender_keys.sender_id \
         WHERE dm_channel_sender_keys.dm_channel_id = $1 AND dm_channel_sender_keys.recipient_id = $2",
    )
    .bind(dm_id)
    .bind(session.user_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows))
}

#[derive(Debug, Serialize)]
pub struct AttachmentSummary {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
}

#[derive(Debug, Serialize)]
pub struct ReplyPreview {
    pub id: Uuid,
    pub author: String,
    pub content: Option<String>,
    pub has_attachment: bool,
}

#[derive(Debug, Serialize)]
pub struct ReactionSummary {
    pub emoji: String,
    pub count: i64,
    pub reacted: bool,
}

#[derive(Debug, Serialize)]
pub struct MessageDto {
    pub id: Uuid,
    pub author_id: Option<Uuid>,
    pub author: String,
    pub content: Option<String>,
    pub attachment: Option<AttachmentSummary>,
    pub reply_to: Option<ReplyPreview>,
    pub reactions: Vec<ReactionSummary>,
    pub pinned: bool,
    pub timestamp: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct MessageRow {
    id: Uuid,
    author_id: Option<Uuid>,
    author: Option<String>,
    encrypted_blob: Option<Vec<u8>>,
    attachment_id: Option<Uuid>,
    attachment_filename: Option<String>,
    attachment_mime_type: Option<String>,
    attachment_size_bytes: Option<i64>,
    reply_to_id: Option<Uuid>,
    pinned: bool,
    timestamp: DateTime<Utc>,
    edited_at: Option<DateTime<Utc>>,
}

const SELECT_MESSAGE_ROW: &str = "SELECT dm_messages.id, dm_messages.author_id, users.username AS author, \
     dm_messages.encrypted_blob, dm_messages.attachment_id, attachments.filename AS attachment_filename, \
     attachments.mime_type AS attachment_mime_type, attachments.size_bytes AS attachment_size_bytes, \
     dm_messages.reply_to_id, dm_messages.pinned, dm_messages.\"timestamp\", dm_messages.edited_at \
     FROM dm_messages \
     LEFT JOIN users ON users.id = dm_messages.author_id \
     LEFT JOIN attachments ON attachments.id = dm_messages.attachment_id";

async fn load_reply_previews(
    pool: &sqlx::PgPool,
    ids: &[Uuid],
) -> Result<HashMap<Uuid, ReplyPreview>, AppError> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        id: Uuid,
        author: Option<String>,
        encrypted_blob: Option<Vec<u8>>,
        has_attachment: bool,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT dm_messages.id, users.username AS author, dm_messages.encrypted_blob, \
                (dm_messages.attachment_id IS NOT NULL) AS has_attachment \
         FROM dm_messages \
         LEFT JOIN users ON users.id = dm_messages.author_id \
         WHERE dm_messages.id = ANY($1)",
    )
    .bind(ids)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            (
                r.id,
                ReplyPreview {
                    id: r.id,
                    author: r.author.unwrap_or_else(|| "unknown".to_string()),
                    content: r.encrypted_blob.map(|b| String::from_utf8_lossy(&b).into_owned()),
                    has_attachment: r.has_attachment,
                },
            )
        })
        .collect())
}

async fn load_reactions(
    pool: &sqlx::PgPool,
    message_ids: &[Uuid],
    requester_id: Uuid,
) -> Result<HashMap<Uuid, Vec<ReactionSummary>>, AppError> {
    if message_ids.is_empty() {
        return Ok(HashMap::new());
    }

    #[derive(sqlx::FromRow)]
    struct Row {
        dm_message_id: Uuid,
        emoji: String,
        count: i64,
        reacted: bool,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT dm_message_id, emoji, COUNT(*) AS count, \
                BOOL_OR(user_id = $2) AS reacted \
         FROM dm_message_reactions \
         WHERE dm_message_id = ANY($1) \
         GROUP BY dm_message_id, emoji \
         ORDER BY MIN(created_at)",
    )
    .bind(message_ids)
    .bind(requester_id)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<ReactionSummary>> = HashMap::new();
    for row in rows {
        map.entry(row.dm_message_id).or_default().push(ReactionSummary {
            emoji: row.emoji,
            count: row.count,
            reacted: row.reacted,
        });
    }
    Ok(map)
}

async fn assemble_messages(
    pool: &sqlx::PgPool,
    rows: Vec<MessageRow>,
    requester_id: Uuid,
) -> Result<Vec<MessageDto>, AppError> {
    let reply_ids: Vec<Uuid> = rows.iter().filter_map(|r| r.reply_to_id).collect();
    let message_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();

    let mut replies = load_reply_previews(pool, &reply_ids).await?;
    let mut reactions = load_reactions(pool, &message_ids, requester_id).await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            let attachment = row.attachment_id.map(|id| AttachmentSummary {
                id,
                filename: row.attachment_filename.unwrap_or_default(),
                mime_type: row.attachment_mime_type.unwrap_or_default(),
                size_bytes: row.attachment_size_bytes.unwrap_or_default(),
            });
            let reply_to = row.reply_to_id.and_then(|id| replies.remove(&id));

            MessageDto {
                id: row.id,
                author_id: row.author_id,
                author: row.author.unwrap_or_else(|| "unknown".to_string()),
                content: row
                    .encrypted_blob
                    .map(|blob| String::from_utf8_lossy(&blob).into_owned()),
                attachment,
                reply_to,
                reactions: reactions.remove(&row.id).unwrap_or_default(),
                pinned: row.pinned,
                timestamp: row.timestamp,
                edited_at: row.edited_at,
            }
        })
        .collect())
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub before: Option<Uuid>,
    pub after: Option<Uuid>,
    pub limit: Option<i64>,
}

pub async fn list_messages(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    async fn timestamp_of(pool: &sqlx::PgPool, id: Uuid) -> Result<DateTime<Utc>, AppError> {
        let row: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT \"timestamp\" FROM dm_messages WHERE id = $1")
                .bind(id)
                .fetch_optional(pool)
                .await?;
        Ok(row.ok_or(AppError::NotFound)?.0)
    }

    let before_ts = match query.before {
        Some(id) => Some(timestamp_of(&state.pool, id).await?),
        None => None,
    };
    let after_ts = match query.after {
        Some(id) => Some(timestamp_of(&state.pool, id).await?),
        None => None,
    };

    let rows: Vec<MessageRow> = if after_ts.is_some() {
        sqlx::query_as(&format!(
            "{SELECT_MESSAGE_ROW} \
             WHERE dm_messages.dm_channel_id = $1 \
               AND dm_messages.\"timestamp\" > $2 \
             ORDER BY dm_messages.\"timestamp\" ASC \
             LIMIT $3"
        ))
        .bind(dm_id)
        .bind(after_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        let mut rows: Vec<MessageRow> = sqlx::query_as(&format!(
            "{SELECT_MESSAGE_ROW} \
             WHERE dm_messages.dm_channel_id = $1 \
               AND ($2::timestamptz IS NULL OR dm_messages.\"timestamp\" < $2) \
             ORDER BY dm_messages.\"timestamp\" DESC \
             LIMIT $3"
        ))
        .bind(dm_id)
        .bind(before_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?;
        rows.reverse();
        rows
    };

    let messages = assemble_messages(&state.pool, rows, session.user_id).await?;
    Ok(Json(messages))
}

pub async fn list_pinned(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    let rows: Vec<MessageRow> = sqlx::query_as(&format!(
        "{SELECT_MESSAGE_ROW} WHERE dm_messages.dm_channel_id = $1 AND dm_messages.pinned = true \
         ORDER BY dm_messages.\"timestamp\" DESC"
    ))
    .bind(dm_id)
    .fetch_all(&state.pool)
    .await?;

    let messages = assemble_messages(&state.pool, rows, session.user_id).await?;
    Ok(Json(messages))
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub attachment_id: Option<Uuid>,
    #[serde(default)]
    pub reply_to_id: Option<Uuid>,
}

async fn require_owned_attachment(
    pool: &sqlx::PgPool,
    attachment_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let owner: Option<(Uuid,)> =
        sqlx::query_as("SELECT uploader_id FROM attachments WHERE id = $1")
            .bind(attachment_id)
            .fetch_optional(pool)
            .await?;

    match owner {
        Some((uploader_id,)) if uploader_id == user_id => Ok(()),
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::AttachmentNotFound),
    }
}

async fn require_same_dm_message(
    pool: &sqlx::PgPool,
    dm_id: Uuid,
    message_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> =
        sqlx::query_as("SELECT dm_channel_id FROM dm_messages WHERE id = $1")
            .bind(message_id)
            .fetch_optional(pool)
            .await?;
    match row {
        Some((cid,)) if cid == dm_id => Ok(()),
        Some(_) => Err(AppError::NotFound),
        None => Err(AppError::NotFound),
    }
}

// --- live message fan-out to the DM's members ---------------------------

async fn broadcast_dm_created(state: &AppState, dm_id: Uuid, dto: &MessageDto) {
    let Ok(message) = serde_json::to_value(dto) else {
        return;
    };
    let members = dm_member_ids(&state.pool, dm_id).await;
    push_to_users(
        state,
        &members,
        &GatewayEvent::MessageCreated {
            context: "dm".to_string(),
            channel_id: dm_id,
            message,
        },
    );
}

async fn broadcast_dm_updated(state: &AppState, dm_id: Uuid, message_id: Uuid) {
    push_to_users(
        state,
        &dm_member_ids(&state.pool, dm_id).await,
        &GatewayEvent::MessageUpdated {
            context: "dm".to_string(),
            channel_id: dm_id,
            message_id,
        },
    );
}

async fn broadcast_dm_deleted(state: &AppState, dm_id: Uuid, message_id: Uuid) {
    push_to_users(
        state,
        &dm_member_ids(&state.pool, dm_id).await,
        &GatewayEvent::MessageDeleted {
            context: "dm".to_string(),
            channel_id: dm_id,
            message_id,
        },
    );
}

pub async fn get_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((dm_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MessageDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;

    let row: MessageRow = sqlx::query_as(&format!("{SELECT_MESSAGE_ROW} WHERE dm_messages.id = $1"))
        .bind(message_id)
        .fetch_one(&state.pool)
        .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    Ok(Json(messages.remove(0)))
}

pub async fn send_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path(dm_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;

    if !state.message_limiter.check(session.user_id) {
        return Err(AppError::RateLimited);
    }

    let content = payload.content.as_deref().map(str::trim).filter(|c| !c.is_empty());

    if content.is_none() && payload.attachment_id.is_none() {
        return Err(AppError::InvalidMessage);
    }
    if let Some(content) = content {
        if content.chars().count() > MAX_CONTENT_LEN {
            return Err(AppError::InvalidMessage);
        }
    }
    if let Some(attachment_id) = payload.attachment_id {
        require_owned_attachment(&state.pool, attachment_id, session.user_id).await?;
    }
    if let Some(reply_to_id) = payload.reply_to_id {
        require_same_dm_message(&state.pool, dm_id, reply_to_id).await?;
    }

    let row: MessageRow = sqlx::query_as(&format!(
        "WITH inserted AS ( \
            INSERT INTO dm_messages (dm_channel_id, author_id, encrypted_blob, attachment_id, reply_to_id) \
            VALUES ($1, $2, $3, $4, $5) \
            RETURNING * \
         ) \
         SELECT inserted.id, inserted.author_id, $6 AS author, \
                inserted.encrypted_blob, inserted.attachment_id, attachments.filename AS attachment_filename, \
                attachments.mime_type AS attachment_mime_type, attachments.size_bytes AS attachment_size_bytes, \
                inserted.reply_to_id, inserted.pinned, inserted.\"timestamp\", inserted.edited_at \
         FROM inserted \
         LEFT JOIN attachments ON attachments.id = inserted.attachment_id"
    ))
    .bind(dm_id)
    .bind(session.user_id)
    .bind(content.map(|c| c.as_bytes()))
    .bind(payload.attachment_id)
    .bind(payload.reply_to_id)
    .bind(&session.username)
    .fetch_one(&state.pool)
    .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    let dto = messages.remove(0);
    broadcast_dm_created(&state, dm_id, &dto).await;
    push_new_dm_notification(&state, dm_id, session.user_id, &session.username).await;
    Ok(Json(dto))
}

/// Web-push the other DM members who have no gateway socket open. Content is
/// E2E-encrypted, so the body is generic ("sent you a message").
async fn push_new_dm_notification(
    state: &AppState,
    dm_id: Uuid,
    sender_id: Uuid,
    sender_username: &str,
) {
    let meta: Option<(bool, Option<String>)> =
        sqlx::query_as("SELECT is_group, name FROM dm_channels WHERE id = $1")
            .bind(dm_id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();
    let (title, body) = match meta {
        Some((true, name)) => (
            name.unwrap_or_else(|| "Group".to_string()),
            format!("{sender_username}: new message"),
        ),
        _ => (sender_username.to_string(), "Sent you a message".to_string()),
    };
    let recipients: Vec<Uuid> = dm_member_ids(&state.pool, dm_id)
        .await
        .into_iter()
        .filter(|u| *u != sender_id)
        .collect();
    crate::push::notify_offline(
        state,
        &recipients,
        crate::push::PushPayload {
            title,
            body,
            url: "/app/".to_string(),
            tag: format!("dm-{dm_id}"),
        },
    );
}

#[derive(Debug, Deserialize)]
pub struct EditMessageRequest {
    pub content: String,
}

async fn require_author(
    pool: &sqlx::PgPool,
    message_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Option<Uuid>,)> =
        sqlx::query_as("SELECT author_id FROM dm_messages WHERE id = $1")
            .bind(message_id)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((Some(author_id),)) if author_id == user_id => Ok(()),
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::NotFound),
    }
}

pub async fn edit_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((dm_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EditMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;
    require_author(&state.pool, message_id, session.user_id).await?;

    let content = payload.content.trim();
    if content.is_empty() || content.chars().count() > MAX_CONTENT_LEN {
        return Err(AppError::InvalidMessage);
    }

    sqlx::query("UPDATE dm_messages SET encrypted_blob = $1, edited_at = now() WHERE id = $2")
        .bind(content.as_bytes())
        .bind(message_id)
        .execute(&state.pool)
        .await?;

    let row: MessageRow = sqlx::query_as(&format!("{SELECT_MESSAGE_ROW} WHERE dm_messages.id = $1"))
        .bind(message_id)
        .fetch_one(&state.pool)
        .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    broadcast_dm_updated(&state, dm_id, message_id).await;
    Ok(Json(messages.remove(0)))
}

pub async fn delete_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((dm_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;
    require_author(&state.pool, message_id, session.user_id).await?;

    sqlx::query("DELETE FROM dm_messages WHERE id = $1")
        .bind(message_id)
        .execute(&state.pool)
        .await?;

    broadcast_dm_deleted(&state, dm_id, message_id).await;

    Ok(())
}

async fn set_pinned(
    state: State<AppState>,
    session: AuthSession,
    path: Path<(Uuid, Uuid)>,
    pinned: bool,
) -> Result<(), AppError> {
    let Path((dm_id, message_id)) = path;
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;

    sqlx::query("UPDATE dm_messages SET pinned = $1 WHERE id = $2")
        .bind(pinned)
        .bind(message_id)
        .execute(&state.pool)
        .await?;

    broadcast_dm_updated(&state, dm_id, message_id).await;

    Ok(())
}

pub async fn pin_message(
    state: State<AppState>,
    session: AuthSession,
    path: Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    set_pinned(state, session, path, true).await
}

pub async fn unpin_message(
    state: State<AppState>,
    session: AuthSession,
    path: Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    set_pinned(state, session, path, false).await
}

pub async fn add_reaction(
    State(state): State<AppState>,
    session: AuthSession,
    Path((dm_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> Result<(), AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;
    validate_emoji(&emoji)?;

    sqlx::query(
        "INSERT INTO dm_message_reactions (dm_message_id, user_id, emoji) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(message_id)
    .bind(session.user_id)
    .bind(&emoji)
    .execute(&state.pool)
    .await?;

    broadcast_dm_updated(&state, dm_id, message_id).await;

    Ok(())
}

pub async fn remove_reaction(
    State(state): State<AppState>,
    session: AuthSession,
    Path((dm_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> Result<(), AppError> {
    require_participant(&state.pool, dm_id, session.user_id).await?;
    require_same_dm_message(&state.pool, dm_id, message_id).await?;

    sqlx::query(
        "DELETE FROM dm_message_reactions WHERE dm_message_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind(message_id)
    .bind(session.user_id)
    .bind(&emoji)
    .execute(&state.pool)
    .await?;

    broadcast_dm_updated(&state, dm_id, message_id).await;

    Ok(())
}
