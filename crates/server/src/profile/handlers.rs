use axum::extract::{Path, State};
use axum::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::gateway::{push_to_friends, GatewayEvent};
use crate::social::user_id_by_username;
use crate::state::AppState;

const MAX_BIO_LEN: usize = 190;
const MAX_STATUS_TEXT_LEN: usize = 128;
const MAX_PRONOUNS_LEN: usize = 40;
const MAX_DISPLAY_NAME_LEN: usize = 32;
const MAX_ACTIVITY_FIELD_LEN: usize = 128;
const MAX_ACTIVITY_IMAGE_LEN: usize = 300;
const MAX_CONNECTION_LABEL_LEN: usize = 64;
const MAX_CONNECTION_URL_LEN: usize = 256;
const MAX_CONNECTIONS_PER_USER: i64 = 5;
const MAX_WIDGET_TITLE_LEN: usize = 48;
const MAX_WIDGETS_PER_USER: i64 = 6;
const MAX_WIDGET_DESCRIPTION_LEN: usize = 140;
const MAX_WIDGET_TAG_LEN: usize = 24;
const MAX_WIDGET_TAGS: usize = 5;
const WIDGET_KINDS: &[&str] = &["favorite_game", "want_to_play", "games_i_like", "games_in_rotation"];

/// Hosts allowed for a widget's external_image_url. Only Steam's own public
/// CDN is trusted here - the game catalog a user picks from client-side is
/// built from real Steam app IDs, so this just confirms the URL actually
/// points at that CDN rather than an arbitrary third-party host.
const WIDGET_IMAGE_HOSTS: &[&str] = &[
    "cdn.cloudflare.steamstatic.com",
    "shared.cloudflare.steamstatic.com",
    "cdn.akamai.steamstatic.com",
];

fn is_allowed_widget_image_host(host: &str) -> bool {
    WIDGET_IMAGE_HOSTS.iter().any(|allowed| host == *allowed)
}

/// Whitelist of connectable services: (id, matching domains). The URL a user
/// submits must resolve to one of the listed domains for that service - this
/// is an allowlist, not a denylist, so nothing outside this set can ever be
/// saved as a connection.
const CONNECTION_SERVICES: &[(&str, &[&str])] = &[
    ("github", &["github.com"]),
    ("youtube", &["youtube.com", "youtu.be"]),
    ("twitch", &["twitch.tv"]),
    ("x", &["x.com", "twitter.com"]),
    ("instagram", &["instagram.com"]),
    ("tiktok", &["tiktok.com"]),
    ("reddit", &["reddit.com"]),
    ("steam", &["steamcommunity.com"]),
    ("spotify", &["spotify.com", "open.spotify.com"]),
    ("discord", &["discord.com", "discord.gg"]),
    ("facebook", &["facebook.com"]),
    ("telegram", &["t.me", "telegram.me"]),
    ("vk", &["vk.com"]),
    ("behance", &["behance.net"]),
    ("dribbble", &["dribbble.com"]),
    ("soundcloud", &["soundcloud.com"]),
    ("bandcamp", &["bandcamp.com"]),
    ("itchio", &["itch.io"]),
    ("xbox", &["xbox.com"]),
    ("playstation", &["playstation.com"]),
    ("battlenet", &["battle.net"]),
    ("epicgames", &["epicgames.com", "store.epicgames.com"]),
    ("roblox", &["roblox.com"]),
];

fn service_for_host(service: &str, host: &str) -> bool {
    CONNECTION_SERVICES
        .iter()
        .find(|(id, _)| *id == service)
        .is_some_and(|(_, domains)| {
            domains
                .iter()
                .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")))
        })
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].chars().all(|c| c.is_ascii_hexdigit())
}

fn file_url(attachment_id: Uuid, filename: &str) -> String {
    use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
    format!(
        "/files/{attachment_id}/{}",
        utf8_percent_encode(filename, NON_ALPHANUMERIC)
    )
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
    pub activity_application: Option<String>,
    pub activity_details: Option<String>,
    pub activity_state: Option<String>,
    pub activity_image: Option<String>,
    pub activity_small_image: Option<String>,
    pub activity_small_text: Option<String>,
    pub activity_started_at: Option<DateTime<Utc>>,
    pub activity_party_size: Option<i16>,
    pub activity_party_max: Option<i16>,
    pub media_application: Option<String>,
    pub media_details: Option<String>,
    pub media_state: Option<String>,
    pub share_activity: bool,
    pub accent_color: Option<String>,
    pub banner_color: Option<String>,
    pub banner_gradient_end: Option<String>,
    pub name_font: Option<String>,
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
    activity_application: Option<String>,
    activity_details: Option<String>,
    activity_state: Option<String>,
    activity_image: Option<String>,
    activity_small_image: Option<String>,
    activity_small_text: Option<String>,
    activity_started_at: Option<DateTime<Utc>>,
    activity_party_size: Option<i16>,
    activity_party_max: Option<i16>,
    media_application: Option<String>,
    media_details: Option<String>,
    media_state: Option<String>,
    share_activity: bool,
    accent_color: Option<String>,
    banner_color: Option<String>,
    banner_gradient_end: Option<String>,
    name_font: Option<String>,
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
                users.presence, users.activity_application, users.activity_details, users.activity_state, \
                users.activity_image, users.activity_small_image, users.activity_small_text, \
                users.activity_started_at, users.activity_party_size, users.activity_party_max, \
                users.media_application, users.media_details, users.media_state, \
                users.share_activity, \
                users.accent_color, users.banner_color, users.banner_gradient_end, users.name_font, users.created_at AS member_since, \
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
        activity_application: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_application)
            .flatten(),
        activity_details: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_details)
            .flatten(),
        activity_state: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_state)
            .flatten(),
        activity_image: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_image)
            .flatten(),
        activity_small_image: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_small_image)
            .flatten(),
        activity_small_text: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_small_text)
            .flatten(),
        activity_started_at: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_started_at)
            .flatten(),
        activity_party_size: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_party_size)
            .flatten(),
        activity_party_max: (row.share_activity || viewer_id == user_id)
            .then_some(row.activity_party_max)
            .flatten(),
        media_application: (row.share_activity || viewer_id == user_id)
            .then_some(row.media_application)
            .flatten(),
        media_details: (row.share_activity || viewer_id == user_id)
            .then_some(row.media_details)
            .flatten(),
        media_state: (row.share_activity || viewer_id == user_id)
            .then_some(row.media_state)
            .flatten(),
        share_activity: row.share_activity,
        accent_color: row.accent_color,
        banner_color: row.banner_color,
        banner_gradient_end: row.banner_gradient_end,
        name_font: row.name_font,
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
    #[serde(default)]
    pub banner_gradient_end: Option<String>,
    #[serde(default)]
    pub name_font: Option<String>,
    #[serde(default)]
    pub share_activity: Option<bool>,
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

/// Allow-list of nameplate font ids. Kept in sync with the client's
/// PRESET_FONT_IDS; anything else is rejected so the value can be dropped
/// straight into a `font-family` on other users' screens.
const NAME_FONTS: &[&str] = &[
    "cinzel",
    "orbitron",
    "unifraktur",
    "pacifico",
    "silkscreen",
    "monoton",
    "rubik-mono",
    "caveat",
];

fn clean_name_font(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = value else { return Ok(None) };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(Some(String::new()));
    }
    if !NAME_FONTS.contains(&trimmed) {
        return Err(AppError::InvalidProfileField);
    }
    Ok(Some(trimmed.to_string()))
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
    let banner_gradient_end = clean_color(payload.banner_gradient_end)?;
    let name_font = clean_name_font(payload.name_font)?;
    // Custom nameplate fonts are a HollowChatter perk; free users may only clear it.
    if name_font.as_deref().is_some_and(|f| !f.is_empty())
        && !crate::billing::is_premium(&state.pool, session.user_id).await?
    {
        return Err(AppError::NotPremium);
    }
    let clear_minutes = payload.status_clear_minutes.filter(|m| *m >= 0);

    sqlx::query(
        "UPDATE users SET \
            display_name = CASE WHEN $1 IS NULL THEN display_name WHEN $1 = '' THEN NULL ELSE $1 END, \
            bio = CASE WHEN $2 IS NULL THEN bio WHEN $2 = '' THEN NULL ELSE $2 END, \
            pronouns = CASE WHEN $3 IS NULL THEN pronouns WHEN $3 = '' THEN NULL ELSE $3 END, \
            status_text = CASE WHEN $4 IS NULL THEN status_text WHEN $4 = '' THEN NULL ELSE $4 END, \
            accent_color = CASE WHEN $5 IS NULL THEN accent_color WHEN $5 = '' THEN NULL ELSE $5 END, \
            banner_color = CASE WHEN $6 IS NULL THEN banner_color WHEN $6 = '' THEN NULL ELSE $6 END, \
            banner_gradient_end = CASE WHEN $10 IS NULL THEN banner_gradient_end WHEN $10 = '' THEN NULL ELSE $10 END, \
            name_font = CASE WHEN $11 IS NULL THEN name_font WHEN $11 = '' THEN NULL ELSE $11 END, \
            share_activity = CASE WHEN $9::boolean IS NULL THEN share_activity ELSE $9::boolean END, \
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
    .bind(payload.share_activity)
    .bind(&banner_gradient_end)
    .bind(&name_font)
    .execute(&state.pool)
    .await?;

    let profile = load_profile(&state.pool, session.user_id, session.user_id).await?;

    if payload.share_activity == Some(false) {
        push_to_friends(
            &state,
            session.user_id,
            &GatewayEvent::PresenceUpdate {
                user_id: session.user_id,
                presence: profile.presence.clone(),
                status_text: profile.status_text.clone(),
                activity_application: None,
                activity_details: None,
                activity_state: None,
                activity_image: None,
                activity_small_image: None,
                activity_small_text: None,
                activity_started_at: None,
                activity_party_size: None,
                activity_party_max: None,
                media_application: None,
                media_details: None,
                media_state: None,
            },
        )
        .await;
    }

    Ok(Json(profile))
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

    let profile = load_profile(&state.pool, session.user_id, session.user_id).await?;

    push_to_friends(
        &state,
        session.user_id,
        &GatewayEvent::PresenceUpdate {
            user_id: session.user_id,
            presence: profile.presence.clone(),
            status_text: profile.status_text.clone(),
            activity_application: profile.share_activity.then(|| profile.activity_application.clone()).flatten(),
            activity_details: profile.share_activity.then(|| profile.activity_details.clone()).flatten(),
            activity_state: profile.share_activity.then(|| profile.activity_state.clone()).flatten(),
            activity_image: profile.share_activity.then(|| profile.activity_image.clone()).flatten(),
            activity_small_image: profile.share_activity.then(|| profile.activity_small_image.clone()).flatten(),
            activity_small_text: profile.share_activity.then(|| profile.activity_small_text.clone()).flatten(),
            activity_started_at: profile.share_activity.then_some(profile.activity_started_at).flatten(),
            activity_party_size: profile.share_activity.then_some(profile.activity_party_size).flatten(),
            activity_party_max: profile.share_activity.then_some(profile.activity_party_max).flatten(),
            media_application: profile.share_activity.then(|| profile.media_application.clone()).flatten(),
            media_details: profile.share_activity.then(|| profile.media_details.clone()).flatten(),
            media_state: profile.share_activity.then(|| profile.media_state.clone()).flatten(),
        },
    )
    .await;

    Ok(Json(profile))
}

#[derive(Debug, Deserialize)]
pub struct SetActivityRequest {
    #[serde(default)]
    pub application: Option<String>,
    #[serde(default)]
    pub details: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub small_image: Option<String>,
    #[serde(default)]
    pub small_text: Option<String>,
    #[serde(default)]
    pub started_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub party_size: Option<i16>,
    #[serde(default)]
    pub party_max: Option<i16>,
    /// "game" (default, from the Discord-IPC bridge) or "media" (from the
    /// browser extension bridge) - the two run independently so watching a
    /// YouTube video doesn't clobber a game's activity or vice versa.
    #[serde(default = "default_activity_kind")]
    pub kind: String,
}

fn default_activity_kind() -> String {
    "game".to_string()
}

/// Called by the desktop client's local Rich Presence bridges (the
/// Discord-IPC game bridge, or the browser extension's media bridge)
/// whenever they report (or clear) an activity. `None`/empty clears that
/// slot, matching a game closing or a video/track stopping.
pub async fn set_activity(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<SetActivityRequest>,
) -> Result<Json<ProfileDto>, AppError> {
    let application = clean(payload.application, MAX_ACTIVITY_FIELD_LEN)?;
    let details = clean(payload.details, MAX_ACTIVITY_FIELD_LEN)?;
    let activity_state = clean(payload.state, MAX_ACTIVITY_FIELD_LEN)?;
    let image = clean(payload.image, MAX_ACTIVITY_IMAGE_LEN)?.filter(|url| url.starts_with("https://"));
    let small_image =
        clean(payload.small_image, MAX_ACTIVITY_IMAGE_LEN)?.filter(|url| url.starts_with("https://"));
    let small_text = clean(payload.small_text, MAX_ACTIVITY_FIELD_LEN)?;
    let is_media = payload.kind == "media";

    if is_media {
        sqlx::query(
            "UPDATE users SET \
                media_application = $1, \
                media_details = $2, \
                media_state = $3 \
             WHERE id = $4",
        )
        .bind(&application)
        .bind(&details)
        .bind(&activity_state)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE users SET \
                activity_application = $1, \
                activity_details = $2, \
                activity_state = $3, \
                activity_image = $4, \
                activity_small_image = $5, \
                activity_small_text = $6, \
                activity_started_at = $7, \
                activity_party_size = $8, \
                activity_party_max = $9 \
             WHERE id = $10",
        )
        .bind(&application)
        .bind(&details)
        .bind(&activity_state)
        .bind(&image)
        .bind(&small_image)
        .bind(&small_text)
        .bind(payload.started_at)
        .bind(payload.party_size)
        .bind(payload.party_max)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;
    }

    let profile = load_profile(&state.pool, session.user_id, session.user_id).await?;

    if profile.share_activity {
        push_to_friends(
            &state,
            session.user_id,
            &GatewayEvent::PresenceUpdate {
                user_id: session.user_id,
                presence: profile.presence.clone(),
                status_text: profile.status_text.clone(),
                activity_application: profile.activity_application.clone(),
                activity_details: profile.activity_details.clone(),
                activity_state: profile.activity_state.clone(),
                activity_image: profile.activity_image.clone(),
                activity_small_image: profile.activity_small_image.clone(),
                activity_small_text: profile.activity_small_text.clone(),
                activity_started_at: profile.activity_started_at,
                activity_party_size: profile.activity_party_size,
                activity_party_max: profile.activity_party_max,
                media_application: profile.media_application.clone(),
                media_details: profile.media_details.clone(),
                media_state: profile.media_state.clone(),
            },
        )
        .await;
    }

    Ok(Json(profile))
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
        // Animated avatars/banners: an image, or a short mp4/webm the client
        // renders as an autoplaying loop.
        Some((mime,))
            if mime.starts_with("image/") || mime == "video/mp4" || mime == "video/webm" =>
        {
            Ok(())
        }
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

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConnectionDto {
    pub id: Uuid,
    pub service: String,
    pub label: String,
    pub url: String,
}

pub async fn list_connections(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Vec<ConnectionDto>>, AppError> {
    let user_id = user_id_by_username(&state.pool, &username).await?;
    let rows: Vec<ConnectionDto> = sqlx::query_as(
        "SELECT id, service, label, url FROM user_connections \
         WHERE user_id = $1 ORDER BY sort_order, created_at",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows))
}

#[derive(Debug, Deserialize)]
pub struct AddConnectionRequest {
    pub service: String,
    #[serde(default)]
    pub label: Option<String>,
    pub url: String,
}

pub async fn add_connection(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<AddConnectionRequest>,
) -> Result<Json<ConnectionDto>, AppError> {
    let raw_url = payload.url.trim();
    if raw_url.is_empty() || raw_url.chars().count() > MAX_CONNECTION_URL_LEN {
        return Err(AppError::InvalidProfileField);
    }

    let parsed = url::Url::parse(raw_url).map_err(|_| AppError::InvalidProfileField)?;
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err(AppError::InvalidProfileField);
    }
    let host = parsed
        .host_str()
        .ok_or(AppError::InvalidProfileField)?
        .to_lowercase();
    let host = host.strip_prefix("www.").unwrap_or(&host);
    if !service_for_host(&payload.service, host) {
        return Err(AppError::LinkNotAllowed);
    }
    let service = payload.service.as_str();

    let label = payload
        .label
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(host);
    if label.chars().count() > MAX_CONNECTION_LABEL_LEN {
        return Err(AppError::InvalidProfileField);
    }
    let url = parsed.as_str();

    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM user_connections WHERE user_id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await?;
    if count >= MAX_CONNECTIONS_PER_USER {
        return Err(AppError::InvalidProfileField);
    }

    let row: ConnectionDto = sqlx::query_as(
        "INSERT INTO user_connections (user_id, service, label, url, sort_order) \
         VALUES ($1, $2, $3, $4, $5) \
         RETURNING id, service, label, url",
    )
    .bind(session.user_id)
    .bind(service)
    .bind(label)
    .bind(url)
    .bind(count as i32)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row))
}

pub async fn remove_connection(
    State(state): State<AppState>,
    session: AuthSession,
    Path(connection_id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM user_connections WHERE id = $1 AND user_id = $2")
        .bind(connection_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WidgetDto {
    pub id: Uuid,
    pub kind: String,
    pub title: String,
    #[sqlx(skip)]
    pub image_url: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub pinned: bool,
}

#[derive(sqlx::FromRow)]
struct WidgetRow {
    id: Uuid,
    kind: String,
    title: String,
    image_attachment_id: Option<Uuid>,
    image_filename: Option<String>,
    external_image_url: Option<String>,
    description: Option<String>,
    tags: Vec<String>,
    pinned: bool,
}

impl From<WidgetRow> for WidgetDto {
    fn from(row: WidgetRow) -> Self {
        WidgetDto {
            id: row.id,
            kind: row.kind,
            title: row.title,
            image_url: row
                .image_attachment_id
                .zip(row.image_filename)
                .map(|(id, name)| file_url(id, &name))
                .or(row.external_image_url),
            description: row.description,
            tags: row.tags,
            pinned: row.pinned,
        }
    }
}

const WIDGET_ROW_COLUMNS: &str = "profile_widgets.id, profile_widgets.kind, profile_widgets.title, \
     profile_widgets.image_attachment_id, attachments.filename AS image_filename, \
     profile_widgets.pinned, \
     profile_widgets.external_image_url, profile_widgets.description, profile_widgets.tags";

pub async fn list_widgets(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Result<Json<Vec<WidgetDto>>, AppError> {
    let user_id = user_id_by_username(&state.pool, &username).await?;
    let rows: Vec<WidgetRow> = sqlx::query_as(&format!(
        "SELECT {WIDGET_ROW_COLUMNS} \
         FROM profile_widgets \
         LEFT JOIN attachments ON attachments.id = profile_widgets.image_attachment_id \
         WHERE profile_widgets.user_id = $1 \
         ORDER BY profile_widgets.pinned DESC, profile_widgets.sort_order, profile_widgets.created_at"
    ))
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(rows.into_iter().map(WidgetDto::from).collect()))
}

fn validate_widget_image_url(url: &str) -> Result<(), AppError> {
    let parsed = url::Url::parse(url).map_err(|_| AppError::LinkNotAllowed)?;
    if parsed.scheme() != "https" {
        return Err(AppError::LinkNotAllowed);
    }
    let host = parsed.host_str().ok_or(AppError::LinkNotAllowed)?;
    if !is_allowed_widget_image_host(host) {
        return Err(AppError::LinkNotAllowed);
    }
    Ok(())
}

fn validate_widget_tags(tags: &[String]) -> Result<Vec<String>, AppError> {
    if tags.len() > MAX_WIDGET_TAGS {
        return Err(AppError::InvalidProfileField);
    }
    tags.iter()
        .map(|tag| {
            let trimmed = tag.trim();
            if trimmed.is_empty() || trimmed.chars().count() > MAX_WIDGET_TAG_LEN {
                return Err(AppError::InvalidProfileField);
            }
            Ok(trimmed.to_string())
        })
        .collect()
}

#[derive(Debug, Deserialize)]
pub struct AddWidgetRequest {
    pub kind: String,
    pub title: String,
    #[serde(default)]
    pub image_attachment_id: Option<Uuid>,
    #[serde(default)]
    pub external_image_url: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub async fn add_widget(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<AddWidgetRequest>,
) -> Result<Json<WidgetDto>, AppError> {
    if !WIDGET_KINDS.contains(&payload.kind.as_str()) {
        return Err(AppError::InvalidProfileField);
    }
    let title = payload.title.trim();
    if title.is_empty() || title.chars().count() > MAX_WIDGET_TITLE_LEN {
        return Err(AppError::InvalidProfileField);
    }
    if let Some(attachment_id) = payload.image_attachment_id {
        owned_image_attachment(&state.pool, session.user_id, attachment_id).await?;
    }
    if let Some(url) = &payload.external_image_url {
        validate_widget_image_url(url)?;
    }
    let description = match &payload.description {
        Some(text) if text.trim().chars().count() > MAX_WIDGET_DESCRIPTION_LEN => {
            return Err(AppError::InvalidProfileField);
        }
        Some(text) if text.trim().is_empty() => None,
        Some(text) => Some(text.trim().to_string()),
        None => None,
    };
    let tags = validate_widget_tags(&payload.tags)?;

    let (count,): (i64,) =
        sqlx::query_as("SELECT count(*) FROM profile_widgets WHERE user_id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await?;
    if count >= MAX_WIDGETS_PER_USER {
        return Err(AppError::InvalidProfileField);
    }

    let row: WidgetRow = sqlx::query_as(&format!(
        "INSERT INTO profile_widgets \
             (user_id, kind, title, image_attachment_id, external_image_url, description, tags, sort_order) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
         RETURNING id, kind, title, image_attachment_id, \
                   (SELECT filename FROM attachments WHERE attachments.id = $4) AS image_filename, \
                   external_image_url, description, tags, pinned"
    ))
    .bind(session.user_id)
    .bind(&payload.kind)
    .bind(title)
    .bind(payload.image_attachment_id)
    .bind(&payload.external_image_url)
    .bind(&description)
    .bind(&tags)
    .bind(count as i32)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row.into()))
}

#[derive(Debug, Deserialize)]
pub struct UpdateWidgetRequest {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub pinned: Option<bool>,
}

pub async fn update_widget(
    State(state): State<AppState>,
    session: AuthSession,
    Path(widget_id): Path<Uuid>,
    Json(payload): Json<UpdateWidgetRequest>,
) -> Result<Json<WidgetDto>, AppError> {
    let description_provided = payload.description.is_some();
    let description = match payload.description {
        Some(text) if text.trim().chars().count() > MAX_WIDGET_DESCRIPTION_LEN => {
            return Err(AppError::InvalidProfileField);
        }
        Some(text) if text.trim().is_empty() => None,
        Some(text) => Some(text.trim().to_string()),
        None => None,
    };
    let tags_provided = payload.tags.is_some();
    let tags = match &payload.tags {
        Some(tags) => validate_widget_tags(tags)?,
        None => Vec::new(),
    };
    let pinned_provided = payload.pinned.is_some();
    let pinned = payload.pinned.unwrap_or(false);

    let updated = sqlx::query(
        "UPDATE profile_widgets SET \
             description = CASE WHEN $3 THEN $4 ELSE description END, \
             tags = CASE WHEN $5 THEN $6 ELSE tags END, \
             pinned = CASE WHEN $7 THEN $8 ELSE pinned END \
         WHERE id = $1 AND user_id = $2",
    )
    .bind(widget_id)
    .bind(session.user_id)
    .bind(description_provided)
    .bind(&description)
    .bind(tags_provided)
    .bind(&tags)
    .bind(pinned_provided)
    .bind(pinned)
    .execute(&state.pool)
    .await?;

    if updated.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    let row: WidgetRow = sqlx::query_as(&format!(
        "SELECT {WIDGET_ROW_COLUMNS} \
         FROM profile_widgets \
         LEFT JOIN attachments ON attachments.id = profile_widgets.image_attachment_id \
         WHERE profile_widgets.id = $1"
    ))
    .bind(widget_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(row.into()))
}

pub async fn remove_widget(
    State(state): State<AppState>,
    session: AuthSession,
    Path(widget_id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM profile_widgets WHERE id = $1 AND user_id = $2")
        .bind(widget_id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }
    Ok(())
}
