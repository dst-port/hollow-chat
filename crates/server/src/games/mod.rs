mod handlers;

use std::sync::Arc;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

#[derive(Clone)]
pub struct GameCoverConfig {
    pub steamgriddb_api_key: Option<Arc<str>>,
}

pub fn router(state: AppState) -> Router<AppState> {
    handlers::spawn_cache_pruner(&state);
    Router::new().route("/cover", get(handlers::cover)).with_state(state)
}
