mod handlers;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{channel_id}/messages", get(handlers::list_messages))
        .route("/{channel_id}/messages", post(handlers::send_message))
        .with_state(state)
}
