use axum::extract::State;
use axum::Json;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
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

    sqlx::query("INSERT INTO sessions (user_id, token_hash, expires_at) VALUES ($1, $2, $3)")
        .bind(user.id)
        .bind(&token_hash)
        .bind(expires_at)
        .execute(&state.pool)
        .await?;

    Ok(Json(LoginResponse { token, expires_at }))
}

#[derive(Debug, Serialize)]
pub struct MeResponse {
    pub username: String,
}

pub async fn me(session: AuthSession) -> Json<MeResponse> {
    Json(MeResponse {
        username: session.username,
    })
}
