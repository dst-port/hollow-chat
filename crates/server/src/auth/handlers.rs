use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;
use crate::password::{generate_password, hash_password, verify_password};
use crate::secret_box;
use crate::session::{generate_token, SESSION_TTL_HOURS};
use crate::state::AppState;
use crate::totp;

use super::extractor::AuthSession;

/// No connection metadata (IP, User-Agent, ...) is ever captured or stored
/// against a session - a database dump (or a memory dump mid-request)
/// should never be able to link an account to a network or a device.
async fn create_session(pool: &sqlx::PgPool, user_id: Uuid) -> Result<LoginResponse, AppError> {
    let (token, token_hash) = generate_token();
    let expires_at = Utc::now() + Duration::hours(SESSION_TTL_HOURS);

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at) \
         VALUES ($1, $2, $3)",
    )
    .bind(user_id)
    .bind(&token_hash)
    .bind(expires_at)
    .execute(pool)
    .await?;

    Ok(LoginResponse {
        requires_totp: false,
        challenge_id: None,
        token: Some(token),
        expires_at: Some(expires_at),
    })
}

fn validate_username(username: &str) -> Result<(), AppError> {
    let len_ok = (3..=32).contains(&username.len());
    let chars_ok = username
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_');
    if len_ok && chars_ok {
        Ok(())
    } else {
        Err(AppError::InvalidUsername)
    }
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub username: String,
    pub password: String,
}

pub async fn register(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, AppError> {
    validate_username(&payload.username)?;

    let password = generate_password();
    let password_hash = hash_password(&password, &state.pepper)?;

    let result = sqlx::query("INSERT INTO users (username, password_hash) VALUES ($1, $2)")
        .bind(&payload.username)
        .bind(&password_hash)
        .execute(&state.pool)
        .await;

    if let Err(sqlx::Error::Database(db_err)) = &result {
        if db_err.is_unique_violation() {
            return Err(AppError::UsernameTaken);
        }
    }
    result?;

    Ok(Json(RegisterResponse {
        username: payload.username,
        password,
    }))
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub requires_totp: bool,
    pub challenge_id: Option<Uuid>,
    pub token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(sqlx::FromRow)]
struct UserAuthRow {
    id: Uuid,
    password_hash: String,
    totp_enabled: bool,
}

pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user: Option<UserAuthRow> = sqlx::query_as(
        "SELECT id, password_hash, totp_enabled FROM users WHERE username = $1",
    )
    .bind(&payload.username)
    .fetch_optional(&state.pool)
    .await?;

    let user = user.ok_or(AppError::InvalidCredentials)?;

    let valid = verify_password(&payload.password, &user.password_hash, &state.pepper)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    if user.totp_enabled {
        let challenge_id: (Uuid,) = sqlx::query_as(
            "INSERT INTO login_challenges (user_id, expires_at) VALUES ($1, $2) RETURNING id",
        )
        .bind(user.id)
        .bind(Utc::now() + Duration::minutes(5))
        .fetch_one(&state.pool)
        .await?;

        return Ok(Json(LoginResponse {
            requires_totp: true,
            challenge_id: Some(challenge_id.0),
            token: None,
            expires_at: None,
        }));
    }

    let response = create_session(&state.pool, user.id).await?;
    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct CompleteTotpLoginRequest {
    pub challenge_id: Uuid,
    pub code: String,
}

pub async fn complete_totp_login(
    State(state): State<AppState>,
    Json(payload): Json<CompleteTotpLoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let challenge: Option<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT user_id, expires_at FROM login_challenges WHERE id = $1",
    )
    .bind(payload.challenge_id)
    .fetch_optional(&state.pool)
    .await?;

    let (user_id, expires_at) = challenge.ok_or(AppError::InvalidCredentials)?;
    if expires_at < Utc::now() {
        sqlx::query("DELETE FROM login_challenges WHERE id = $1")
            .bind(payload.challenge_id)
            .execute(&state.pool)
            .await?;
        return Err(AppError::InvalidCredentials);
    }

    verify_totp_or_backup(&state, user_id, &payload.code).await?;

    sqlx::query("DELETE FROM login_challenges WHERE id = $1")
        .bind(payload.challenge_id)
        .execute(&state.pool)
        .await?;

    let response = create_session(&state.pool, user_id).await?;
    Ok(Json(response))
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub id: Uuid,
    pub username: String,
}

pub async fn me(session: AuthSession) -> Json<MeResponse> {
    Json(MeResponse {
        id: session.user_id,
        username: session.username,
    })
}

#[derive(Debug, Deserialize)]
pub struct ChangeUsernameRequest {
    pub username: String,
}

#[derive(Debug, Serialize)]
pub struct ChangeUsernameResponse {
    pub username: String,
}

pub async fn change_username(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<ChangeUsernameRequest>,
) -> Result<Json<ChangeUsernameResponse>, AppError> {
    validate_username(&payload.username)?;

    let result = sqlx::query("UPDATE users SET username = $1 WHERE id = $2")
        .bind(&payload.username)
        .bind(session.user_id)
        .execute(&state.pool)
        .await;

    if let Err(sqlx::Error::Database(db_err)) = &result {
        if db_err.is_unique_violation() {
            return Err(AppError::UsernameTaken);
        }
    }
    result?;

    Ok(Json(ChangeUsernameResponse {
        username: payload.username,
    }))
}

#[derive(Debug, Serialize)]
pub struct RegeneratePasswordResponse {
    pub password: String,
}

pub async fn regenerate_password(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<RegeneratePasswordResponse>, AppError> {
    let password = generate_password();
    let password_hash = hash_password(&password, &state.pepper)?;

    sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
        .bind(&password_hash)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(Json(RegeneratePasswordResponse { password }))
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SessionDto {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub current: bool,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<SessionDto>>, AppError> {
    let rows: Vec<(Uuid, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, created_at FROM sessions \
         WHERE user_id = $1 AND expires_at > $2 \
         ORDER BY created_at DESC",
    )
    .bind(session.user_id)
    .bind(Utc::now())
    .fetch_all(&state.pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|(id, created_at)| SessionDto {
            id,
            created_at,
            current: id == session.session_id,
        })
        .collect();

    Ok(Json(sessions))
}

fn generate_backup_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut rng = rand::thread_rng();
    let part = |rng: &mut rand::rngs::ThreadRng| -> String {
        (0..4)
            .map(|_| ALPHABET[rng.gen_range(0..ALPHABET.len())] as char)
            .collect()
    };
    format!("{}-{}", part(&mut rng), part(&mut rng))
}

async fn verify_totp_or_backup(state: &AppState, user_id: Uuid, code: &str) -> Result<(), AppError> {
    let secret: Option<(Option<String>,)> =
        sqlx::query_as("SELECT totp_secret FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;

    let Some((Some(sealed),)) = secret else {
        return Err(AppError::TotpNotConfigured);
    };
    let secret = secret_box::open(&state.pepper, &sealed)?;

    if totp::verify_code(&secret, code) {
        return Ok(());
    }

    let backup_codes: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT id, code_hash FROM totp_backup_codes WHERE user_id = $1 AND used_at IS NULL",
    )
    .bind(user_id)
    .fetch_all(&state.pool)
    .await?;

    for (id, hash) in backup_codes {
        if verify_password(code, &hash, &state.pepper)? {
            sqlx::query("UPDATE totp_backup_codes SET used_at = now() WHERE id = $1")
                .bind(id)
                .execute(&state.pool)
                .await?;
            return Ok(());
        }
    }

    Err(AppError::InvalidTotpCode)
}

async fn generate_and_store_backup_codes(
    state: &AppState,
    user_id: Uuid,
) -> Result<Vec<String>, AppError> {
    sqlx::query("DELETE FROM totp_backup_codes WHERE user_id = $1")
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    let mut codes = Vec::with_capacity(8);
    for _ in 0..8 {
        let code = generate_backup_code();
        let hash = hash_password(&code, &state.pepper)?;
        sqlx::query("INSERT INTO totp_backup_codes (user_id, code_hash) VALUES ($1, $2)")
            .bind(user_id)
            .bind(&hash)
            .execute(&state.pool)
            .await?;
        codes.push(code);
    }

    Ok(codes)
}

#[derive(Debug, Serialize)]
pub struct TotpStatusResponse {
    pub enabled: bool,
}

pub async fn totp_status(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<TotpStatusResponse>, AppError> {
    let row: (bool,) = sqlx::query_as("SELECT totp_enabled FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(TotpStatusResponse { enabled: row.0 }))
}

#[derive(Debug, Serialize)]
pub struct TotpSetupResponse {
    pub secret: String,
    pub otpauth_url: String,
}

pub async fn totp_setup(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<TotpSetupResponse>, AppError> {
    let secret = totp::generate_secret();
    let sealed = secret_box::seal(&state.pepper, &secret);

    sqlx::query("UPDATE users SET totp_secret = $1, totp_enabled = false WHERE id = $2")
        .bind(&sealed)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    let otpauth_url = totp::otpauth_url(&secret, &session.username, "HollowChat");

    Ok(Json(TotpSetupResponse { secret, otpauth_url }))
}

#[derive(Debug, Deserialize)]
pub struct TotpCodeRequest {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct TotpVerifyResponse {
    pub backup_codes: Vec<String>,
}

pub async fn totp_verify(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<TotpCodeRequest>,
) -> Result<Json<TotpVerifyResponse>, AppError> {
    let secret: (Option<String>,) = sqlx::query_as("SELECT totp_secret FROM users WHERE id = $1")
        .bind(session.user_id)
        .fetch_one(&state.pool)
        .await?;

    let sealed = secret.0.ok_or(AppError::TotpNotConfigured)?;
    let secret = secret_box::open(&state.pepper, &sealed)?;

    if !totp::verify_code(&secret, &payload.code) {
        return Err(AppError::InvalidTotpCode);
    }

    sqlx::query("UPDATE users SET totp_enabled = true WHERE id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    let backup_codes = generate_and_store_backup_codes(&state, session.user_id).await?;

    Ok(Json(TotpVerifyResponse { backup_codes }))
}

pub async fn totp_disable(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<TotpCodeRequest>,
) -> Result<(), AppError> {
    verify_totp_or_backup(&state, session.user_id, &payload.code).await?;

    sqlx::query("UPDATE users SET totp_secret = NULL, totp_enabled = false WHERE id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;
    sqlx::query("DELETE FROM totp_backup_codes WHERE user_id = $1")
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}

pub async fn totp_regenerate_backup_codes(
    State(state): State<AppState>,
    session: AuthSession,
    Json(payload): Json<TotpCodeRequest>,
) -> Result<Json<TotpVerifyResponse>, AppError> {
    verify_totp_or_backup(&state, session.user_id, &payload.code).await?;
    let backup_codes = generate_and_store_backup_codes(&state, session.user_id).await?;
    Ok(Json(TotpVerifyResponse { backup_codes }))
}

pub async fn revoke_session(
    State(state): State<AppState>,
    session: AuthSession,
    axum::extract::Path(id): axum::extract::Path<Uuid>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = $1 AND user_id = $2")
        .bind(id)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}
