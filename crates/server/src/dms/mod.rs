mod handlers;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_dms))
        .route("/", post(handlers::open_dm))
        .route("/{id}/messages", get(handlers::list_messages))
        .route("/{id}/messages", post(handlers::send_message))
        .with_state(state)
}
