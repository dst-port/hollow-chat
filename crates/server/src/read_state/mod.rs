mod handlers;

use axum::routing::{get, put};
use axum::Router;

use crate::state::AppState;

/// Full-path routes merged at the app root (they span /channels, /dms and
/// /unreads, so this isn't a single nest).
pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/channels/{channel_id}/read", put(handlers::mark_channel_read))
        .route("/dms/{dm_id}/read", put(handlers::mark_dm_read))
        .route("/unreads", get(handlers::get_unreads))
        .with_state(state)
}
