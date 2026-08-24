mod handlers;

use axum::routing::{delete, get, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_blocked))
        .route("/{user_id}", put(handlers::block_user))
        .route("/{user_id}", delete(handlers::unblock_user))
        .with_state(state)
}
