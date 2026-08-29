mod handlers;

use axum::routing::{get, patch, post};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_report))
        .route("/", get(handlers::list_reports))
        .route("/{id}/sealed", get(handlers::get_report_sealed))
        .route("/{id}/status", patch(handlers::update_report_status))
        .with_state(state)
}
