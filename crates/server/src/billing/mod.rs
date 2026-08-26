mod handlers;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub use handlers::{is_premium, PREMIUM_BOOST_SLOTS};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/checkout", post(handlers::create_checkout))
        .route("/webhook", post(handlers::webhook))
        .route("/status", get(handlers::status))
        .with_state(state)
}
