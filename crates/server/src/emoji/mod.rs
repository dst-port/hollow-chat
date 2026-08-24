mod handlers;

use axum::routing::{get, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/use", post(handlers::record_use))
        .route("/frequent", get(handlers::frequent))
        .with_state(state)
}
