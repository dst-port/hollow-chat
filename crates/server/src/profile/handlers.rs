use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::social::user_id_by_username;
use crate::state::AppState;

const MAX_BIO_LEN: usize = 190;
const MAX_STATUS_TEXT_LEN: usize = 128;
const MAX_PRONOUNS_LEN: usize = 40;
const MAX_DISPLAY_NAME_LEN: usize = 32;

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn file_url(attachment_id: Uuid, filename: &str) -> String {
    format!("/files/{attachment_id}/{filename}")
}

const PRESENCE_VALUES: &[&str] = &["online", "idle", "dnd", "invisible"];

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProfileDto {
    pub username: String,
    pub display_name: Option<String>,
    pub bio: Option<String>,
    pub pronouns: Option<String>,
    pub status_text: Option<String>,
    pub presence: String,
    pub accent_color: Option<String>,
    pub banner_color: Option<String>,
    #[sqlx(skip)]
    pub avatar_url: Option<String>,
    #[sqlx(skip)]
    pub banner_url: Option<String>,
    pub member_since: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct ProfileRow {
    username: String,
    display_name: Option<String>,
    bio: Option<String>,
    pronouns: Option<String>,
    status_text: Option<String>,
    presence: String,
    accent_color: Option<String>,
    banner_color: Option<String>,
    member_since: DateTime<Utc>,
    avatar_attachment_id: Option<Uuid>,
    avatar_filename: Option<String>,
    banner_attachment_id: Option<Uuid>,
    banner_filename: Option<String>,
}

async fn load_profile(
    pool: &sqlx::PgPool,
    user_id: Uuid,
    viewer_id: Uuid,
) -> Result<ProfileDto, AppError> {
    let row: ProfileRow = sqlx::query_as(
        "SELECT users.username, users.display_name, users.bio, users.pronouns, \
                CASE WHEN users.status_clear_at IS NOT NULL AND users.status_clear_at < now() \
                     THEN NULL ELSE users.status_text END AS status_text, \
                users.presence, users.accent_color, users.banner_color, users.created_at AS member_since, \
                users.avatar_attachment_id, avatar.filename AS avatar_filename, \
                users.banner_attachment_id, banner.filename AS banner_filename \
         FROM users \
         LEFT JOIN attachments avatar ON avatar.id = users.avatar_attachment_id \
         LEFT JOIN attachments banner ON banner.id = users.banner_attachment_id \
         WHERE users.id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(ProfileDto {
        username: row.username,
        display_name: row.display_name,
        bio: row.bio,
        pronouns: row.pronouns,
        status_text: row.status_text,
        presence: if row.presence == "invisible" && viewer_id != user_id {
            "offline".to_string()
        } else {
            row.presence
        },
        accent_color: row.accent_color,
        banner_color: row.banner_color,
        avatar_url: row
            .avatar_attachment_id
            .zip(row.avatar_filename)
            .map(|(id, name)| file_url(id, &name)),
        banner_url: row
            .banner_attachment_id
            .zip(row.banner_filename)
            .map(|(id, name)| file_url(id, &name)),
        member_since: row.member_since,
    })
}

pub async fn get_profile(
    State(state): State<AppState>,
    session: AuthSession,
    Path(username): Path<String>,
) -> Result<Json<ProfileDto>, AppError> {
    let user_id = user_id_by_username(&state.pool, &username).await?;
    Ok(Json(load_profile(&state.pool, user_id, session.user_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub bio: Option<String>,
    #[serde(default)]
    pub pronouns: Option<String>,
    #[serde(default)]
    pub status_text: Option<String>,
    #[serde(default)]
    pub status_clear_minutes: Option<i64>,
    #[serde(default)]
    pub accent_color: Option<String>,
    #[serde(default)]
    pub banner_color: Option<String>,
}

fn clean(value: Option<String>, max_len: usize) -> Result<Option<String>, AppError> {
    let Some(value) = value else { return Ok(None) };
    let trimmed = value.trim();
    if trimmed.chars().count() > max_len {
        return Err(AppError::InvalidProfileField);
    }
    Ok(if trimmed.is_empty() {
        Some(String::new())
    } else {
        Some(trimmed.to_string())
    })
}

fn clean_color(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value else { return Ok(None) };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Some(String::new()));
    }
    if !is_hex_color(trimmed) {
        return Err(AppError::InvalidProfileField);
    }
    Ok(Some(trimmed.to_string()))
}

pub async fn update_profile(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileDto>, AppError> {
    let display_name = clean(payload.display_name, MAX_DISPLAY_NAME_LEN)?;
    let bio = clean(payload.bio, MAX_BIO_LEN)?;
    let pronouns = clean(payload.pronouns, MAX_PRONOUNS_LEN)?;
    let status_text = clean(payload.status_text, MAX_STATUS_TEXT_LEN)?;
    let accent_color = clean_color(payload.accent_color)?;
    let banner_color = clean_color(payload.banner_color)?;
    let clear_minutes = payload.status_clear_minutes.filter(|m| *m >= 0);

    sqlx::query(
        "UPDATE users SET \
            display_name = CASE WHEN $1 IS NULL THEN display_name WHEN $1 = '' THEN NULL ELSE $1 END, \
            bio = CASE WHEN $2 IS NULL THEN bio WHEN $2 = '' THEN NULL ELSE $2 END, \
            pronouns = CASE WHEN $3 IS NULL THEN pronouns WHEN $3 = '' THEN NULL ELSE $3 END, \
            status_text = CASE WHEN $4 IS NULL THEN status_text WHEN $4 = '' THEN NULL ELSE $4 END, \
            accent_color = CASE WHEN $5 IS NULL THEN accent_color WHEN $5 = '' THEN NULL ELSE $5 END, \
            banner_color = CASE WHEN $6 IS NULL THEN banner_color WHEN $6 = '' THEN NULL ELSE $6 END, \
            status_clear_at = CASE WHEN $8::bigint IS NULL THEN status_clear_at \
                                    WHEN $8::bigint = 0 THEN NULL \
                                    ELSE now() + make_interval(mins => $8::int) END \
         WHERE id = $7",
    )
    .bind(&display_name)
    .bind(&bio)
    .bind(&pronouns)
    .bind(&status_text)
    .bind(&accent_color)
    .bind(&banner_color)
    .bind(session.user_id)
    .bind(clear_minutes)
    .execute(&state.pool)
    .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct SetPresenceRequest {
    pub presence: String,
}

pub async fn set_presence(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<SetPresenceRequest>,
) -> Result<Json<ProfileDto>, AppError> {
    if !PRESENCE_VALUES.contains(&payload.presence.as_str()) {
        return Err(AppError::InvalidProfileField);
    }

    sqlx::query("UPDATE users SET presence = $1 WHERE id = $2")
        .bind(&payload.presence)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}

#[derive(Debug, Deserialize)]
pub struct SetImageRequest {
    pub attachment_id: Uuid,
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

pub async fn set_avatar(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<SetImageRequest>,
) -> Result<Json<ProfileDto>, AppError> {
    owned_image_attachment(&state.pool, session.user_id, payload.attachment_id).await?;

    sqlx::query("UPDATE users SET avatar_attachment_id = $1 WHERE id = $2")
        .bind(payload.attachment_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}

pub async fn clear_avatar(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<ProfileDto>, AppError> {
    sqlx::query("UPDATE users SET avatar_attachment_id = NULL WHERE id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}

pub async fn set_banner(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<SetImageRequest>,
) -> Result<Json<ProfileDto>, AppError> {
    owned_image_attachment(&state.pool, session.user_id, payload.attachment_id).await?;

    sqlx::query("UPDATE users SET banner_attachment_id = $1 WHERE id = $2")
        .bind(payload.attachment_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}

pub async fn clear_banner(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<ProfileDto>, AppError> {
    sqlx::query("UPDATE users SET banner_attachment_id = NULL WHERE id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(load_profile(&state.pool, session.user_id, session.user_id).await?))
}
