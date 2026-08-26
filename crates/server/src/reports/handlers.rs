use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

const MAX_SEALED_KEY_LEN: usize = 512;
const MAX_PAYLOAD_LEN: usize = 10 * 1024 * 1024;

#[derive(Debug, Deserialize)]
pub struct SealedBoxDto {
    pub ephemeral_public: String,
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateReportRequest {
    pub reported_user_id: Uuid,
    pub context_kind: String,
    pub context_id: Uuid,
    pub server_id: Option<Uuid>,
    pub sealed_key: SealedBoxDto,
    pub payload_nonce: String,
    pub payload_ciphertext: String,
}

fn decode(field: &str) -> Result<Vec<u8>, AppError> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(field)
        .map_err(|_| AppError::InvalidReport)
}

async fn require_dm_participant(
    pool: &sqlx::PgPool,
    dm_id: Uuid,
    reporter_id: Uuid,
    reported_user_id: Uuid,
) -> Result<(), AppError> {
    let row: Option<(Uuid, Uuid)> =
        sqlx::query_as("SELECT user_a, user_b FROM dm_channels WHERE id = $1")
            .bind(dm_id)
            .fetch_optional(pool)
            .await?;

    match row {
        Some((a, b)) if (a == reporter_id && b == reported_user_id) || (b == reporter_id && a == reported_user_id) => {
            Ok(())
        }
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::NotFound),
    }
}

async fn require_channel_member(
    pool: &sqlx::PgPool,
    channel_id: Uuid,
    server_id: Uuid,
    reporter_id: Uuid,
) -> Result<(), AppError> {
    let channel_server: Option<(Uuid,)> =
        sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
            .bind(channel_id)
            .fetch_optional(pool)
            .await?;

    match channel_server {
        Some((sid,)) if sid == server_id => {}
        Some(_) => return Err(AppError::InvalidReport),
        None => return Err(AppError::NotFound),
    }

    let member: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(reporter_id)
    .fetch_optional(pool)
    .await?;

    member.map(|_| ()).ok_or(AppError::Unauthorized)
}

#[derive(Debug, serde::Serialize)]
pub struct CreateReportResponse {
    pub id: Uuid,
}

pub async fn create_report(
    State(state): State<AppState>,
    session: AuthSession,
    Json(body): Json<CreateReportRequest>,
) -> Result<Json<CreateReportResponse>, AppError> {
    if body.reported_user_id == session.user_id {
        return Err(AppError::InvalidReport);
    }

    match body.context_kind.as_str() {
        "dm" => {
            require_dm_participant(&state.pool, body.context_id, session.user_id, body.reported_user_id)
                .await?;
        }
        "channel" => {
            let server_id = body.server_id.ok_or(AppError::InvalidReport)?;
            require_channel_member(&state.pool, body.context_id, server_id, session.user_id).await?;
        }
        _ => return Err(AppError::InvalidReport),
    }

    let ephemeral_public = decode(&body.sealed_key.ephemeral_public)?;
    let sealed_nonce = decode(&body.sealed_key.nonce)?;
    let sealed_ciphertext = decode(&body.sealed_key.ciphertext)?;
    let payload_nonce = decode(&body.payload_nonce)?;
    let payload_ciphertext = decode(&body.payload_ciphertext)?;

    if ephemeral_public.len() != 32
        || sealed_ciphertext.len() > MAX_SEALED_KEY_LEN
        || payload_ciphertext.is_empty()
        || payload_ciphertext.len() > MAX_PAYLOAD_LEN
    {
        return Err(AppError::InvalidReport);
    }

    let id: (Uuid,) = sqlx::query_as(
        "INSERT INTO reports \
         (reporter_id, reported_user_id, context_kind, context_id, server_id, \
          sealed_key_ephemeral_public, sealed_key_nonce, sealed_key_ciphertext, \
          payload_nonce, payload_ciphertext) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10) \
         RETURNING id",
    )
    .bind(session.user_id)
    .bind(body.reported_user_id)
    .bind(&body.context_kind)
    .bind(body.context_id)
    .bind(body.server_id)
    .bind(ephemeral_public)
    .bind(sealed_nonce)
    .bind(sealed_ciphertext)
    .bind(payload_nonce)
    .bind(payload_ciphertext)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CreateReportResponse { id: id.0 }))
}
