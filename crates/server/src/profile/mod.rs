mod handlers;

use axum::routing::{get, patch, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", patch(handlers::update_profile))
        .route("/presence", put(handlers::set_presence))
        .route("/avatar", put(handlers::set_avatar).delete(handlers::clear_avatar))
        .route("/banner", put(handlers::set_banner).delete(handlers::clear_banner))
        .route("/{username}", get(handlers::get_profile))
        .with_state(state)
}
