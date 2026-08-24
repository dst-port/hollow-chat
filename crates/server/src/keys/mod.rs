mod handlers;

use axum::routing::{get, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/bundle", put(handlers::upload_bundle))
        .route("/bundle/{username}", get(handlers::fetch_bundle))
        .route("/prekey-count", get(handlers::prekey_count))
        .with_state(state)
}
