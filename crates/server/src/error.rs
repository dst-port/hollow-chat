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
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("unauthorized")]
    Unauthorized,
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
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
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
