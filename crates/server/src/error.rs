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
    #[error("blocked")]
    Blocked,
    #[error("friend request already sent")]
    RequestAlreadySent,
    #[error("invalid key material")]
    InvalidKeyMaterial,
    #[error("no key bundle for this user")]
    NoKeyBundle,
    #[error("another of your devices just started a session with this user, retry shortly")]
    BundleFetchInProgress,
    #[error("you're sending messages too fast, slow down")]
    RateLimited,
    #[error("this channel is in slowmode, wait before sending again")]
    SlowModeActive,
    #[error("file too large for your plan")]
    FileTooLarge,
    #[error("unsupported or missing attachment")]
    InvalidAttachment,
    #[error("attachment not found")]
    AttachmentNotFound,
    #[error("this attachment has expired and is no longer available")]
    AttachmentExpired,
    #[error("invalid profile field")]
    InvalidProfileField,
    #[error("this link isn't allowed")]
    LinkNotAllowed,
    #[error("couldn't load a preview for this link")]
    LinkPreviewUnavailable,
    #[error("billing is not configured on this server")]
    BillingNotConfigured,
    #[error("billing provider error")]
    BillingProvider,
    #[error("couldn't generate an invite code, try again")]
    InviteGenerationFailed,
    #[error("invalid invite code")]
    InvalidInviteCode,
    #[error("void shards are a premium perk")]
    NotPremium,
    #[error("all your boost slots are already assigned")]
    BoostSlotsFull,
    #[error("this server already has all the custom emoji slots its boost level allows")]
    EmojiSlotsFull,
    #[error("that emoji name is already used on this server")]
    EmojiNameTaken,
    #[error("invalid emoji name")]
    InvalidEmojiName,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("invalid or expired verification code")]
    InvalidTotpCode,
    #[error("two-factor authentication is not set up yet")]
    TotpNotConfigured,
    #[error("unauthorized")]
    Unauthorized,
    #[error("invalid report")]
    InvalidReport,
    #[error("not found")]
    NotFound,
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("password hashing error")]
    Hashing,
    #[error("internal error")]
    Internal,
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
            AppError::Blocked => StatusCode::FORBIDDEN,
            AppError::RequestAlreadySent => StatusCode::CONFLICT,
            AppError::InvalidKeyMaterial => StatusCode::BAD_REQUEST,
            AppError::NoKeyBundle => StatusCode::NOT_FOUND,
            AppError::BundleFetchInProgress => StatusCode::CONFLICT,
            AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
            AppError::SlowModeActive => StatusCode::TOO_MANY_REQUESTS,
            AppError::FileTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            AppError::InvalidAttachment => StatusCode::BAD_REQUEST,
            AppError::AttachmentNotFound => StatusCode::NOT_FOUND,
            AppError::AttachmentExpired => StatusCode::GONE,
            AppError::InvalidProfileField => StatusCode::BAD_REQUEST,
            AppError::LinkNotAllowed => StatusCode::BAD_REQUEST,
            AppError::LinkPreviewUnavailable => StatusCode::BAD_GATEWAY,
            AppError::BillingNotConfigured => StatusCode::SERVICE_UNAVAILABLE,
            AppError::BillingProvider => StatusCode::BAD_GATEWAY,
            AppError::InviteGenerationFailed => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::InvalidInviteCode => StatusCode::BAD_REQUEST,
            AppError::NotPremium => StatusCode::FORBIDDEN,
            AppError::BoostSlotsFull => StatusCode::CONFLICT,
            AppError::EmojiSlotsFull => StatusCode::CONFLICT,
            AppError::EmojiNameTaken => StatusCode::CONFLICT,
            AppError::InvalidEmojiName => StatusCode::BAD_REQUEST,
            AppError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            AppError::InvalidTotpCode => StatusCode::BAD_REQUEST,
            AppError::TotpNotConfigured => StatusCode::BAD_REQUEST,
            AppError::Unauthorized => StatusCode::UNAUTHORIZED,
            AppError::InvalidReport => StatusCode::BAD_REQUEST,
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Hashing => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        if let AppError::Database(err) = &self {
            tracing::error!("database error: {err}");
            crate::telegram::notify(format!("HollowChat database error:\n{err}"));
        }
        if matches!(&self, AppError::Hashing | AppError::Internal) {
            tracing::error!("{self}");
            crate::telegram::notify(format!("HollowChat internal error: {self}"));
        }

        let body = Json(json!({ "error": self.to_string() }));
        (status, body).into_response()
    }
}
