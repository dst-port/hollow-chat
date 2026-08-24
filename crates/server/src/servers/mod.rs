mod handlers;

use axum::routing::{delete, get, patch, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_server))
        .route("/", get(handlers::list_servers))
        .route("/{id}", patch(handlers::rename_server))
        .route("/{id}/leave", delete(handlers::leave_server))
        .route("/{id}/channels", post(handlers::create_channel))
        .route("/{id}/members", get(handlers::list_members))
        .with_state(state)
}
