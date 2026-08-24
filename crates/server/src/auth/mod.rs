mod extractor;
mod handlers;

pub use extractor::AuthSession;

use axum::middleware;
use axum::routing::{delete, get, patch, post};
use axum::Router;
use std::time::Duration;

use crate::rate_limit::{self, RateLimiter};
use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    let limiter = RateLimiter::new(5, Duration::from_secs(60));

    Router::new()
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login))
        .layer(middleware::from_fn_with_state(limiter, rate_limit::enforce))
        .route("/me", get(handlers::me))
        .route("/username", patch(handlers::change_username))
        .route("/regenerate-password", post(handlers::regenerate_password))
        .route("/sessions", get(handlers::list_sessions))
        .route("/sessions/{id}", delete(handlers::revoke_session))
        .with_state(state)
}
