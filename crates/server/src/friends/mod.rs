mod handlers;

use axum::routing::{delete, get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_friends))
        .route("/mutual/{username}", get(handlers::list_mutual_friends))
        .route("/{user_id}", delete(handlers::remove_friend))
        .route("/requests", get(handlers::list_requests))
        .route("/requests", post(handlers::send_request))
        .route("/requests/{id}/accept", post(handlers::accept_request))
        .route("/requests/{id}", delete(handlers::decline_request))
        .with_state(state)
}
