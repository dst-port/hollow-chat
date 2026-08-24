mod handlers;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{channel_id}/sender-keys", post(handlers::publish_sender_keys))
        .route("/{channel_id}/sender-keys", get(handlers::list_sender_keys))
        .with_state(state)
}
