use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("username already taken")]
    UsernameTaken,
    #[error("invalid username")]
    InvalidUsername,
    #[error("invalid emoji")]
    InvalidEmoji,
    #[error("invalid name")]
    InvalidName,
    #[error("invalid channel type")]
    InvalidChannelType,
    #[error("invalid message")]
    InvalidMessage,
    #[error("already friends")]
    AlreadyFriends,
    #[error("friend request already sent")]
    RequestAlreadySent,
    #[error("invalid key material")]
    InvalidKeyMaterial,
    #[error("no key bundle for this user")]
    NoKeyBundle,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
    #[error("not found")]
    NotFound,
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("password hashing error")]
    Hashing,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::UsernameTaken => StatusCode::CONFLICT,
            AppError::InvalidUsername => StatusCode::BAD_REQUEST,
            AppError::InvalidEmoji => StatusCode::BAD_REQUEST,
            AppError::InvalidName => StatusCode::BAD_REQUEST,
            AppError::InvalidChannelType => StatusCode::BAD_REQUEST,
            AppError::InvalidMessage => StatusCode::BAD_REQUEST,
            AppError::AlreadyFriends => StatusCode::CONFLICT,
            AppError::RequestAlreadySent => StatusCode::CONFLICT,
            AppError::InvalidKeyMaterial => StatusCode::BAD_REQUEST,
            AppError::NoKeyBundle => StatusCode::NOT_FOUND,
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Hashing => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if let AppError::Database(err) = &self {
            tracing::error!("database error: {err}");
        }

        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
