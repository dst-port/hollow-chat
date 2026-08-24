mod handlers;

use axum::routing::get;
use axum::Router;

use crate::state::AppState;

pub use handlers::{award, revoke};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::catalog))
        .route("/{username}", get(handlers::for_user))
        .with_state(state)
}
