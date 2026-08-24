use axum::extract::{ConnectInfo, State};
use axum::http::header::USER_AGENT;
use axum::http::HeaderMap;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

use crate::error::AppError;
use crate::password::{generate_password, hash_password, verify_password};
use crate::session::{generate_token, SESSION_TTL_HOURS};
use crate::state::AppState;

use super::extractor::AuthSession;

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
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct UserAuthRow {
    id: Uuid,
    password_hash: String,
}

pub async fn login(
    State(state): State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user: Option<UserAuthRow> =
        sqlx::query_as("SELECT id, password_hash FROM users WHERE username = $1")
            .bind(&payload.username)
            .fetch_optional(&state.pool)
            .await?;

    let user = user.ok_or(AppError::InvalidCredentials)?;

    let valid = verify_password(&payload.password, &user.password_hash, &state.pepper)?;
    if !valid {
        return Err(AppError::InvalidCredentials);
    }

    let (token, token_hash) = generate_token();
    let expires_at = Utc::now() + Duration::hours(SESSION_TTL_HOURS);
    let user_agent = headers
        .get(USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown");
    let ip_address = addr.ip().to_string();

    sqlx::query(
        "INSERT INTO sessions (user_id, token_hash, expires_at, user_agent, ip_address) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(user.id)
    .bind(&token_hash)
    .bind(expires_at)
    .bind(user_agent)
    .bind(&ip_address)
    .execute(&state.pool)
    .await?;

    Ok(Json(LoginResponse { token, expires_at }))
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
    pub user_agent: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: DateTime<Utc>,
    pub current: bool,
}

pub async fn list_sessions(
    State(state): State<AppState>,
    session: AuthSession,
) -> Result<Json<Vec<SessionDto>>, AppError> {
    let rows: Vec<(Uuid, Option<String>, Option<String>, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, user_agent, ip_address, created_at FROM sessions \
         WHERE user_id = $1 AND expires_at > $2 \
         ORDER BY created_at DESC",
    )
    .bind(session.user_id)
    .bind(Utc::now())
    .fetch_all(&state.pool)
    .await?;

    let sessions = rows
        .into_iter()
        .map(|(id, user_agent, ip_address, created_at)| SessionDto {
            id,
            user_agent,
            ip_address,
            created_at,
            current: id == session.session_id,
        })
        .collect();

    Ok(Json(sessions))
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
