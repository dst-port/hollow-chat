use axum::extract::{Path, State};
use axum::Json;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::social::user_id_by_username;
use crate::state::AppState;

fn decode(field: &str) -> Result<Vec<u8>, AppError> {
    STANDARD.decode(field).map_err(|_| AppError::InvalidKeyMaterial)
}

fn encode(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}

#[derive(Debug, Deserialize)]
pub struct OneTimePrekeyUpload {
    pub key_id: i32,
    pub public_key: String,
}

#[derive(Debug, Deserialize)]
pub struct UploadBundleRequest {
    pub ed25519_public: String,
    pub x25519_public: String,
    pub signed_prekey_id: i32,
    pub signed_prekey_public: String,
    pub signed_prekey_signature: String,
    #[serde(default)]
    pub one_time_prekeys: Vec<OneTimePrekeyUpload>,
}

pub async fn upload_bundle(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<UploadBundleRequest>,
) -> Result<(), AppError> {
    let ed25519_public = decode(&payload.ed25519_public)?;
    let x25519_public = decode(&payload.x25519_public)?;
    let signed_prekey_public = decode(&payload.signed_prekey_public)?;
    let signed_prekey_signature = decode(&payload.signed_prekey_signature)?;

    if ed25519_public.len() != 32 || x25519_public.len() != 32 || signed_prekey_public.len() != 32
    {
        return Err(AppError::InvalidKeyMaterial);
    }

    let mut tx = state.pool.begin().await?;

    let existing: Option<(Vec<u8>,)> =
        sqlx::query_as("SELECT x25519_public FROM identity_keys WHERE user_id = $1")
            .bind(session.user_id)
            .fetch_optional(&mut *tx)
            .await?;

    let identity_changed = existing.map(|(x,)| x != x25519_public).unwrap_or(true);

    sqlx::query(
        "INSERT INTO identity_keys \
            (user_id, ed25519_public, x25519_public, signed_prekey_id, signed_prekey_public, signed_prekey_signature, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, now()) \
         ON CONFLICT (user_id) DO UPDATE SET \
            ed25519_public = EXCLUDED.ed25519_public, \
            x25519_public = EXCLUDED.x25519_public, \
            signed_prekey_id = EXCLUDED.signed_prekey_id, \
            signed_prekey_public = EXCLUDED.signed_prekey_public, \
            signed_prekey_signature = EXCLUDED.signed_prekey_signature, \
            updated_at = now()",
    )
    .bind(session.user_id)
    .bind(&ed25519_public)
    .bind(&x25519_public)
    .bind(payload.signed_prekey_id)
    .bind(&signed_prekey_public)
    .bind(&signed_prekey_signature)
    .execute(&mut *tx)
    .await?;

    if identity_changed {
        sqlx::query("DELETE FROM one_time_prekeys WHERE user_id = $1")
            .bind(session.user_id)
            .execute(&mut *tx)
            .await?;
    }

    for prekey in &payload.one_time_prekeys {
        let public_key = decode(&prekey.public_key)?;
        if public_key.len() != 32 {
            return Err(AppError::InvalidKeyMaterial);
        }
        sqlx::query(
            "INSERT INTO one_time_prekeys (user_id, key_id, public_key) \
             VALUES ($1, $2, $3) \
             ON CONFLICT (user_id, key_id) DO NOTHING",
        )
        .bind(session.user_id)
        .bind(prekey.key_id)
        .bind(&public_key)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    Ok(())
}

#[derive(Debug, Serialize)]
pub struct PrekeyBundleResponse {
    pub ed25519_public: String,
    pub x25519_public: String,
    pub signed_prekey_id: i32,
    pub signed_prekey_public: String,
    pub signed_prekey_signature: String,
    pub one_time_prekey: Option<OneTimePrekeyDto>,
}

#[derive(Debug, Serialize)]
pub struct OneTimePrekeyDto {
    pub key_id: i32,
    pub public_key: String,
}

#[derive(sqlx::FromRow)]
struct IdentityRow {
    ed25519_public: Vec<u8>,
    x25519_public: Vec<u8>,
    signed_prekey_id: i32,
    signed_prekey_public: Vec<u8>,
    signed_prekey_signature: Vec<u8>,
}

pub async fn fetch_bundle(
    State(state): State<AppState>,
    _session: AuthSession,
    Path(username): Path<String>,
) -> Result<Json<PrekeyBundleResponse>, AppError> {
    let user_id = user_id_by_username(&state.pool, &username).await?;

    let identity: Option<IdentityRow> = sqlx::query_as(
        "SELECT ed25519_public, x25519_public, signed_prekey_id, signed_prekey_public, signed_prekey_signature \
         FROM identity_keys WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    let identity = identity.ok_or(AppError::NoKeyBundle)?;

    let mut tx = state.pool.begin().await?;

    let claimed: Option<(uuid::Uuid, i32, Vec<u8>)> = sqlx::query_as(
        "SELECT id, key_id, public_key FROM one_time_prekeys \
         WHERE user_id = $1 AND used = false \
         ORDER BY created_at \
         LIMIT 1 FOR UPDATE SKIP LOCKED",
    )
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    let one_time_prekey = if let Some((id, key_id, public_key)) = &claimed {
        sqlx::query("UPDATE one_time_prekeys SET used = true WHERE id = $1")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        Some(OneTimePrekeyDto {
            key_id: *key_id,
            public_key: encode(public_key),
        })
    } else {
        None
    };

    tx.commit().await?;

    Ok(Json(PrekeyBundleResponse {
        ed25519_public: encode(&identity.ed25519_public),
        x25519_public: encode(&identity.x25519_public),
        signed_prekey_id: identity.signed_prekey_id,
        signed_prekey_public: encode(&identity.signed_prekey_public),
        signed_prekey_signature: encode(&identity.signed_prekey_signature),
        one_time_prekey,
    }))
}

#[derive(Debug, Serialize)]
pub struct PrekeyCountResponse {
    pub count: i64,
}

pub async fn prekey_count(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<PrekeyCountResponse>, AppError> {
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM one_time_prekeys WHERE user_id = $1 AND used = false",
    )
    .bind(session.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(PrekeyCountResponse { count: count.0 }))
}
