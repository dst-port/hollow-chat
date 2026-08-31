use axum::body::Body;
use axum::extract::{Multipart, Path, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use uuid::Uuid;

use crate::attachments::bunny;
use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

const FREE_LIMIT: u64 = 50 * 1024 * 1024;
const PREMIUM_LIMIT: u64 = 2 * 1024 * 1024 * 1024;

async fn tier_limit(pool: &sqlx::PgPool, user_id: Uuid) -> Result<u64, AppError> {
    let row: Option<(String,)> = sqlx::query_as("SELECT tier FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
    let tier = row.map(|r| r.0).unwrap_or_else(|| "free".to_string());
    Ok(if tier == "premium" { PREMIUM_LIMIT } else { FREE_LIMIT })
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AttachmentDto {
    pub id: Uuid,
    pub filename: String,
    pub mime_type: String,
    pub size_bytes: i64,
}

pub async fn upload(
    State(state): State<AppState>,
    session: AuthSession,
    mut multipart: Multipart,
) -> Result<Json<AttachmentDto>, AppError> {
    let limit = tier_limit(&state.pool, session.user_id).await?;

    let mut field = multipart
        .next_field()
        .await
        .map_err(|_| AppError::InvalidAttachment)?
        .ok_or(AppError::InvalidAttachment)?;

    let filename = field.file_name().unwrap_or("file").to_string();
    let mime_type = field
        .content_type()
        .unwrap_or("application/octet-stream")
        .to_string();

    let storage_key = Uuid::new_v4().to_string();
    let path = std::path::Path::new(state.attachments_dir.as_ref()).join(&storage_key);

    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(|_| AppError::InvalidAttachment)?;
    let mut total: u64 = 0;

    loop {
        let chunk = match field.chunk().await {
            Ok(Some(chunk)) => chunk,
            Ok(None) => break,
            Err(_) => {
                drop(file);
                let _ = tokio::fs::remove_file(&path).await;
                return Err(AppError::InvalidAttachment);
            }
        };

        total += chunk.len() as u64;
        if total > limit {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(AppError::FileTooLarge);
        }

        if file.write_all(&chunk).await.is_err() {
            drop(file);
            let _ = tokio::fs::remove_file(&path).await;
            return Err(AppError::InvalidAttachment);
        }
    }

    if total == 0 {
        let _ = tokio::fs::remove_file(&path).await;
        return Err(AppError::InvalidAttachment);
    }

    let on_cdn = if let Some(bunny) = &state.bunny {
        let upload_file = tokio::fs::File::open(&path)
            .await
            .map_err(|_| AppError::InvalidAttachment)?;
        let body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(upload_file));
        let uploaded = bunny::upload(&state.http_client, bunny, &storage_key, body).await;
        // The local temp copy was only ever a write buffer once Bunny is
        // configured - drop it either way so a successful CDN upload
        // doesn't double up disk usage, and a failed one doesn't leave
        // orphaned partial files lying around.
        let _ = tokio::fs::remove_file(&path).await;
        if uploaded.is_err() {
            return Err(AppError::InvalidAttachment);
        }
        true
    } else {
        false
    };

    let attachment: AttachmentDto = sqlx::query_as(
        "INSERT INTO attachments (uploader_id, filename, mime_type, size_bytes, storage_key, on_cdn) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         RETURNING id, filename, mime_type, size_bytes",
    )
    .bind(session.user_id)
    .bind(&filename)
    .bind(&mime_type)
    .bind(total as i64)
    .bind(&storage_key)
    .bind(on_cdn)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(attachment))
}

#[derive(sqlx::FromRow)]
struct AttachmentRow {
    uploader_id: Uuid,
    filename: String,
    mime_type: String,
    storage_key: String,
    purged_at: Option<chrono::DateTime<chrono::Utc>>,
    on_cdn: bool,
}

/// Parse a single-range `Range: bytes=start-end` header against a known total.
/// Returns the inclusive byte range, or None for "no/!bytes/multi-range" (serve
/// whole), or Err for an unsatisfiable range.
fn parse_range(headers: &HeaderMap, total: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(raw) = headers.get(header::RANGE).and_then(|v| v.to_str().ok()) else {
        return Ok(None);
    };
    let Some(spec) = raw.strip_prefix("bytes=") else {
        return Ok(None);
    };
    if spec.contains(',') {
        return Ok(None); // multi-range: not worth it, send the whole file
    }
    let (a, b) = spec.split_once('-').ok_or(())?;
    let (start, end) = match (a.trim(), b.trim()) {
        ("", "") => return Err(()),
        ("", suffix) => {
            let n: u64 = suffix.parse().map_err(|_| ())?;
            (total.saturating_sub(n), total.saturating_sub(1))
        }
        (s, "") => (s.parse().map_err(|_| ())?, total.saturating_sub(1)),
        (s, e) => (
            s.parse().map_err(|_| ())?,
            e.parse::<u64>().map_err(|_| ())?.min(total.saturating_sub(1)),
        ),
    };
    if total == 0 || start > end || start >= total {
        return Err(());
    }
    Ok(Some((start, end)))
}

pub async fn download(
    State(state): State<AppState>,
    session: AuthSession,
    headers: HeaderMap,
    Path((id, _filename)): Path<(Uuid, String)>,
) -> Result<Response, AppError> {
    let attachment: Option<AttachmentRow> = sqlx::query_as(
        "SELECT uploader_id, filename, mime_type, storage_key, purged_at, on_cdn FROM attachments WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?;

    let attachment = attachment.ok_or(AppError::AttachmentNotFound)?;
    if attachment.purged_at.is_some() {
        return Err(AppError::AttachmentExpired);
    }

    if attachment.uploader_id != session.user_id {
        let authorized: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM messages \
             JOIN channels ON channels.id = messages.channel_id \
             JOIN server_members ON server_members.server_id = channels.server_id \
             WHERE messages.attachment_id = $1 AND server_members.user_id = $2 \
             UNION \
             SELECT 1 FROM dm_messages \
             JOIN dm_channels ON dm_channels.id = dm_messages.dm_channel_id \
             WHERE dm_messages.attachment_id = $1 \
               AND (dm_channels.user_a = $2 OR dm_channels.user_b = $2) \
             UNION \
             SELECT 1 FROM users \
             WHERE users.avatar_attachment_id = $1 OR users.banner_attachment_id = $1 \
             LIMIT 1",
        )
        .bind(id)
        .bind(session.user_id)
        .fetch_optional(&state.pool)
        .await?;

        if authorized.is_none() {
            return Err(AppError::AttachmentNotFound);
        }
    }

    if attachment.on_cdn {
        if let Some(bunny) = &state.bunny {
            let url = bunny::public_url(bunny, &attachment.storage_key);
            return Ok(axum::response::Redirect::temporary(&url).into_response());
        }
    }

    let path = std::path::Path::new(state.attachments_dir.as_ref()).join(&attachment.storage_key);
    let mut file = tokio::fs::File::open(&path)
        .await
        .map_err(|_| AppError::AttachmentNotFound)?;
    let total = file
        .metadata()
        .await
        .map_err(|_| AppError::AttachmentNotFound)?
        .len();

    // Media elements (<video>/<audio>) issue Range requests and refetch the
    // whole file when the server ignores them - honour a single byte range.
    let range = parse_range(&headers, total);
    if range == Err(()) {
        return Ok(Response::builder()
            .status(StatusCode::RANGE_NOT_SATISFIABLE)
            .header(header::CONTENT_RANGE, format!("bytes */{total}"))
            .body(Body::empty())
            .unwrap());
    }

    // Inline-render anything the browser can display in place; only real
    // downloads (docs, archives) get "attachment".
    let disposition_kind = if attachment.mime_type.starts_with("image/")
        || attachment.mime_type.starts_with("video/")
        || attachment.mime_type.starts_with("audio/")
    {
        "inline"
    } else {
        "attachment"
    };
    let disposition = format!(
        "{disposition_kind}; filename=\"{}\"",
        attachment.filename.replace('"', "")
    );

    let mut builder = Response::builder()
        .header(header::CONTENT_TYPE, attachment.mime_type)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CACHE_CONTROL, "private, max-age=31536000, immutable")
        .header(header::ACCEPT_RANGES, "bytes");

    let (status, body) = match range {
        Ok(Some((start, end))) => {
            let len = end - start + 1;
            file.seek(std::io::SeekFrom::Start(start))
                .await
                .map_err(|_| AppError::AttachmentNotFound)?;
            builder = builder
                .header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{total}"))
                .header(header::CONTENT_LENGTH, len);
            (
                StatusCode::PARTIAL_CONTENT,
                Body::from_stream(tokio_util::io::ReaderStream::new(file.take(len))),
            )
        }
        _ => {
            builder = builder.header(header::CONTENT_LENGTH, total);
            (
                StatusCode::OK,
                Body::from_stream(tokio_util::io::ReaderStream::new(file)),
            )
        }
    };

    Ok(builder.status(status).body(body).unwrap())
}
