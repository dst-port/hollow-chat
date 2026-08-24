mod handlers;

use axum::extract::DefaultBodyLimit;
use axum::routing::{get, put};
use axum::Router;

use crate::state::AppState;

const MAX_BODY_BYTES: usize = 2 * 1024 * 1024 * 1024 + 1024 * 1024;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", put(handlers::upload))
        .route("/{id}/{filename}", get(handlers::download))
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .with_state(state)
}
