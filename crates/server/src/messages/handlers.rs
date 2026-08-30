use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::gateway::{channel_member_ids, push_to_users, GatewayEvent};
use crate::permissions::{require_permission, MANAGE_MESSAGES};
use crate::state::AppState;

const DEFAULT_LIMIT: i64 = 50;
const MAX_LIMIT: i64 = 100;
const MAX_CONTENT_LEN: usize = 4000;
const MAX_EMOJI_LEN: usize = 32;

async fn require_channel_member(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?;

    let (server_id,) = row.ok_or(AppError::NotFound)?;

    let member: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    member.map(|_| ()).ok_or(AppError::Unauthorized)
}

async fn channel_server_id(pool: &sqlx::PgPool, channel_id: Uuid) -> Result<Uuid, AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
        .bind(channel_id)
        .fetch_optional(pool)
        .await?;
    Ok(row.ok_or(AppError::NotFound)?.0)
}

async fn require_slowmode_clear(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(i32,)> =
        sqlx::query_as("SELECT slowmode_seconds FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await?;

    let slowmode_seconds = row.ok_or(AppError::NotFound)?.0;
    if slowmode_seconds <= 0 {
        return Ok(());
    }

    let last: Option<(DateTime<Utc>,)> = sqlx::query_as(
        "SELECT \"timestamp\" FROM messages \
         WHERE channel_id = $1 AND author_id = $2 AND thread_id IS NULL \
         ORDER BY \"timestamp\" DESC LIMIT 1",
    )
    .bind(channel_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some((last_ts,)) = last {
        let elapsed = Utc::now().signed_duration_since(last_ts).num_seconds();
        if elapsed < slowmode_seconds as i64 {
            return Err(AppError::SlowModeActive);
        }
    }

    Ok(())
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

const SELECT_MESSAGE_ROW: &str = "SELECT messages.id, messages.author_id, users.username AS author, \
     messages.encrypted_blob, messages.attachment_id, attachments.filename AS attachment_filename, \
     attachments.mime_type AS attachment_mime_type, attachments.size_bytes AS attachment_size_bytes, \
     messages.reply_to_id, messages.pinned, messages.\"timestamp\", messages.edited_at \
     FROM messages \
     LEFT JOIN users ON users.id = messages.author_id \
     LEFT JOIN attachments ON attachments.id = messages.attachment_id";

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
        "SELECT messages.id, users.username AS author, messages.encrypted_blob, \
                (messages.attachment_id IS NOT NULL) AS has_attachment \
         FROM messages \
         LEFT JOIN users ON users.id = messages.author_id \
         WHERE messages.id = ANY($1)",
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
        message_id: Uuid,
        emoji: String,
        count: i64,
        reacted: bool,
    }

    let rows: Vec<Row> = sqlx::query_as(
        "SELECT message_id, emoji, COUNT(*) AS count, \
                BOOL_OR(user_id = $2) AS reacted \
         FROM message_reactions \
         WHERE message_id = ANY($1) \
         GROUP BY message_id, emoji \
         ORDER BY MIN(created_at)",
    )
    .bind(message_ids)
    .bind(requester_id)
    .fetch_all(pool)
    .await?;

    let mut map: HashMap<Uuid, Vec<ReactionSummary>> = HashMap::new();
    for row in rows {
        map.entry(row.message_id).or_default().push(ReactionSummary {
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

// --- live message fan-out to the channel's server members ----------------

async fn broadcast_created(state: &AppState, channel_id: Uuid, dto: &MessageDto) {
    let Ok(message) = serde_json::to_value(dto) else {
        return;
    };
    let members = channel_member_ids(&state.pool, channel_id).await;
    push_to_users(
        state,
        &members,
        &GatewayEvent::MessageCreated {
            context: "channel".to_string(),
            channel_id,
            message,
        },
    );
}

async fn broadcast_updated(state: &AppState, channel_id: Uuid, message_id: Uuid) {
    let members = channel_member_ids(&state.pool, channel_id).await;
    push_to_users(
        state,
        &members,
        &GatewayEvent::MessageUpdated {
            context: "channel".to_string(),
            channel_id,
            message_id,
        },
    );
}

async fn broadcast_deleted(state: &AppState, channel_id: Uuid, message_id: Uuid) {
    let members = channel_member_ids(&state.pool, channel_id).await;
    push_to_users(
        state,
        &members,
        &GatewayEvent::MessageDeleted {
            context: "channel".to_string(),
            channel_id,
            message_id,
        },
    );
}

pub async fn get_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<MessageDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let row: Option<MessageRow> = sqlx::query_as(&format!(
        "{SELECT_MESSAGE_ROW} WHERE messages.id = $1 AND messages.channel_id = $2"
    ))
    .bind(message_id)
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await?;

    let row = row.ok_or(AppError::NotFound)?;
    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    Ok(Json(messages.remove(0)))
}

pub async fn list_messages(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);

    async fn timestamp_of(pool: &sqlx::PgPool, id: Uuid) -> Result<DateTime<Utc>, AppError> {
        let row: Option<(DateTime<Utc>,)> =
            sqlx::query_as("SELECT \"timestamp\" FROM messages WHERE id = $1")
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
             WHERE messages.channel_id = $1 \
               AND messages.thread_id IS NULL \
               AND messages.\"timestamp\" > $2 \
             ORDER BY messages.\"timestamp\" ASC \
             LIMIT $3"
        ))
        .bind(channel_id)
        .bind(after_ts)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        let mut rows: Vec<MessageRow> = sqlx::query_as(&format!(
            "{SELECT_MESSAGE_ROW} \
             WHERE messages.channel_id = $1 \
               AND messages.thread_id IS NULL \
               AND ($2::timestamptz IS NULL OR messages.\"timestamp\" < $2) \
             ORDER BY messages.\"timestamp\" DESC \
             LIMIT $3"
        ))
        .bind(channel_id)
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
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let rows: Vec<MessageRow> = sqlx::query_as(&format!(
        "{SELECT_MESSAGE_ROW} WHERE messages.channel_id = $1 AND messages.pinned = true \
         ORDER BY messages.\"timestamp\" DESC"
    ))
    .bind(channel_id)
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
    /// Usernames the client resolved to user ids from the message's @mentions,
    /// sent alongside the ciphertext so we can push-notify offline mentionees.
    /// The server can't scan the (E2E-encrypted) content itself. Not stored.
    #[serde(default)]
    pub mentioned_user_ids: Vec<Uuid>,
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

async fn require_same_channel_message(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    message_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT channel_id FROM messages WHERE id = $1")
        .bind(message_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((cid,)) if cid == channel_id => Ok(()),
        Some(_) => Err(AppError::NotFound),
        None => Err(AppError::NotFound),
    }
}

pub async fn send_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_slowmode_clear(&state.pool, channel_id, session.user_id).await?;

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
        require_same_channel_message(&state.pool, channel_id, reply_to_id).await?;
    }

    let row: MessageRow = sqlx::query_as(&format!(
        "WITH inserted AS ( \
            INSERT INTO messages (channel_id, author_id, encrypted_blob, attachment_id, reply_to_id) \
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
    .bind(channel_id)
    .bind(session.user_id)
    .bind(content.map(|c| c.as_bytes()))
    .bind(payload.attachment_id)
    .bind(payload.reply_to_id)
    .bind(&session.username)
    .fetch_one(&state.pool)
    .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    let dto = messages.remove(0);
    broadcast_created(&state, channel_id, &dto).await;
    push_mention_notifications(
        &state,
        channel_id,
        session.user_id,
        &session.username,
        &payload.mentioned_user_ids,
    )
    .await;
    Ok(Json(dto))
}

/// Web-push the mentioned users who belong to the channel's server and have no
/// gateway socket open. Mention ids come from the client (content is E2E).
async fn push_mention_notifications(
    state: &AppState,
    channel_id: Uuid,
    author_id: Uuid,
    author_username: &str,
    mentioned: &[Uuid],
) {
    if mentioned.is_empty() {
        return;
    }
    let members = channel_member_ids(&state.pool, channel_id).await;
    let recipients: Vec<Uuid> = mentioned
        .iter()
        .copied()
        .filter(|u| *u != author_id && members.contains(u))
        .collect();
    if recipients.is_empty() {
        return;
    }
    let meta: Option<(String, String)> = sqlx::query_as(
        "SELECT channels.name, servers.name \
         FROM channels JOIN servers ON servers.id = channels.server_id \
         WHERE channels.id = $1",
    )
    .bind(channel_id)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();
    let (title, body) = match meta {
        Some((channel_name, server_name)) => (
            format!("#{channel_name} · {server_name}"),
            format!("{author_username} mentioned you"),
        ),
        None => (
            "New mention".to_string(),
            format!("{author_username} mentioned you"),
        ),
    };
    crate::push::notify_offline(
        state,
        &recipients,
        crate::push::PushPayload {
            title,
            body,
            url: "/app/".to_string(),
            tag: format!("mention-{channel_id}"),
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
        sqlx::query_as("SELECT author_id FROM messages WHERE id = $1")
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
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<EditMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;
    require_author(&state.pool, message_id, session.user_id).await?;

    let content = payload.content.trim();
    if content.is_empty() || content.chars().count() > MAX_CONTENT_LEN {
        return Err(AppError::InvalidMessage);
    }

    sqlx::query(
        "UPDATE messages SET encrypted_blob = $1, edited_at = now() WHERE id = $2",
    )
    .bind(content.as_bytes())
    .bind(message_id)
    .execute(&state.pool)
    .await?;

    let row: MessageRow = sqlx::query_as(&format!("{SELECT_MESSAGE_ROW} WHERE messages.id = $1"))
        .bind(message_id)
        .fetch_one(&state.pool)
        .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    broadcast_updated(&state, channel_id, message_id).await;
    Ok(Json(messages.remove(0)))
}

pub async fn delete_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
) -> Result<(), AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;

    if require_author(&state.pool, message_id, session.user_id).await.is_err() {
        let server_id = channel_server_id(&state.pool, channel_id).await?;
        require_permission(&state.pool, server_id, session.user_id, MANAGE_MESSAGES).await?;
    }

    sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(message_id)
        .execute(&state.pool)
        .await?;

    broadcast_deleted(&state, channel_id, message_id).await;

    Ok(())
}

pub async fn set_pinned(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
    pinned: bool,
) -> Result<(), AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;
    let server_id = channel_server_id(&state.pool, channel_id).await?;
    require_permission(&state.pool, server_id, session.user_id, MANAGE_MESSAGES).await?;

    sqlx::query("UPDATE messages SET pinned = $1 WHERE id = $2")
        .bind(pinned)
        .bind(message_id)
        .execute(&state.pool)
        .await?;

    broadcast_updated(&state, channel_id, message_id).await;

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
    Path((channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> Result<(), AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;
    validate_emoji(&emoji)?;

    sqlx::query(
        "INSERT INTO message_reactions (message_id, user_id, emoji) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(message_id)
    .bind(session.user_id)
    .bind(&emoji)
    .execute(&state.pool)
    .await?;

    broadcast_updated(&state, channel_id, message_id).await;

    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ThreadDto {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub parent_message_id: Option<Uuid>,
    pub name: String,
    pub created_by: Option<Uuid>,
    pub created_by_username: Option<String>,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub message_count: i64,
    pub last_message_at: Option<DateTime<Utc>>,
}

const SELECT_THREAD_ROW: &str = "SELECT threads.id, threads.channel_id, threads.parent_message_id, \
     threads.name, threads.created_by, users.username AS created_by_username, threads.archived, \
     threads.created_at, \
     COUNT(messages.id) AS message_count, \
     MAX(messages.\"timestamp\") AS last_message_at \
     FROM threads \
     LEFT JOIN users ON users.id = threads.created_by \
     LEFT JOIN messages ON messages.thread_id = threads.id \
     GROUP BY threads.id, users.username";

pub async fn list_threads(
    State(state): State<AppState>,
    session: AuthSession,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<Vec<ThreadDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    let threads: Vec<ThreadDto> = sqlx::query_as(&format!(
        "{SELECT_THREAD_ROW} HAVING threads.channel_id = $1 \
         ORDER BY MAX(messages.\"timestamp\") DESC NULLS LAST, threads.created_at DESC"
    ))
    .bind(channel_id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(threads))
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    #[serde(default)]
    pub name: Option<String>,
}

pub async fn create_thread(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, message_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<CreateThreadRequest>,
) -> Result<Json<ThreadDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;

    let name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .unwrap_or("Thread")
        .chars()
        .take(48)
        .collect::<String>();

    let thread_id: (Uuid,) = sqlx::query_as(
        "INSERT INTO threads (channel_id, parent_message_id, name, created_by) \
         VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(channel_id)
    .bind(message_id)
    .bind(&name)
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await?;

    let thread: ThreadDto = sqlx::query_as(&format!("{SELECT_THREAD_ROW} HAVING threads.id = $1"))
        .bind(thread_id.0)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(thread))
}

#[derive(Debug, Deserialize)]
pub struct SetThreadArchivedRequest {
    pub archived: bool,
}

pub async fn set_thread_archived(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SetThreadArchivedRequest>,
) -> Result<Json<ThreadDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;

    sqlx::query("UPDATE threads SET archived = $1 WHERE id = $2 AND channel_id = $3")
        .bind(payload.archived)
        .bind(thread_id)
        .bind(channel_id)
        .execute(&state.pool)
        .await?;

    let thread: ThreadDto = sqlx::query_as(&format!("{SELECT_THREAD_ROW} HAVING threads.id = $1"))
        .bind(thread_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(thread))
}

async fn require_same_channel_thread(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    thread_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid,)> = sqlx::query_as("SELECT channel_id FROM threads WHERE id = $1")
        .bind(thread_id)
        .fetch_optional(pool)
        .await?;
    match row {
        Some((cid,)) if cid == channel_id => Ok(()),
        Some(_) => Err(AppError::NotFound),
        None => Err(AppError::NotFound),
    }
}

pub async fn list_thread_messages(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<MessageDto>>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_thread(&state.pool, channel_id, thread_id).await?;

    let rows: Vec<MessageRow> = sqlx::query_as(&format!(
        "{SELECT_MESSAGE_ROW} WHERE messages.thread_id = $1 ORDER BY messages.\"timestamp\" ASC"
    ))
    .bind(thread_id)
    .fetch_all(&state.pool)
    .await?;

    let messages = assemble_messages(&state.pool, rows, session.user_id).await?;
    Ok(Json(messages))
}

pub async fn send_thread_message(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, thread_id)): Path<(Uuid, Uuid)>,
    Json(payload): Json<SendMessageRequest>,
) -> Result<Json<MessageDto>, AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_thread(&state.pool, channel_id, thread_id).await?;

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
        require_same_channel_message(&state.pool, channel_id, reply_to_id).await?;
    }

    let row: MessageRow = sqlx::query_as(&format!(
        "WITH inserted AS ( \
            INSERT INTO messages (channel_id, thread_id, author_id, encrypted_blob, attachment_id, reply_to_id) \
            VALUES ($1, $2, $3, $4, $5, $6) \
            RETURNING * \
         ) \
         SELECT inserted.id, inserted.author_id, $7 AS author, \
                inserted.encrypted_blob, inserted.attachment_id, attachments.filename AS attachment_filename, \
                attachments.mime_type AS attachment_mime_type, attachments.size_bytes AS attachment_size_bytes, \
                inserted.reply_to_id, inserted.pinned, inserted.\"timestamp\", inserted.edited_at \
         FROM inserted \
         LEFT JOIN attachments ON attachments.id = inserted.attachment_id"
    ))
    .bind(channel_id)
    .bind(thread_id)
    .bind(session.user_id)
    .bind(content.map(|c| c.as_bytes()))
    .bind(payload.attachment_id)
    .bind(payload.reply_to_id)
    .bind(&session.username)
    .fetch_one(&state.pool)
    .await?;

    let mut messages = assemble_messages(&state.pool, vec![row], session.user_id).await?;
    Ok(Json(messages.remove(0)))
}

pub async fn remove_reaction(
    State(state): State<AppState>,
    session: AuthSession,
    Path((channel_id, message_id, emoji)): Path<(Uuid, Uuid, String)>,
) -> Result<(), AppError> {
    require_channel_member(&state.pool, channel_id, session.user_id).await?;
    require_same_channel_message(&state.pool, channel_id, message_id).await?;

    sqlx::query(
        "DELETE FROM message_reactions WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind(message_id)
    .bind(session.user_id)
    .bind(&emoji)
    .execute(&state.pool)
    .await?;

    broadcast_updated(&state, channel_id, message_id).await;

    Ok(())
}
