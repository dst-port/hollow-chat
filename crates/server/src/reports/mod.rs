mod handlers;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_report))
        .with_state(state)
}
