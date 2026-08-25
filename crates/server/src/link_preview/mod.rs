mod handlers;
mod ssrf;

use axum::routing::post;
use axum::Router;

use crate::state::AppState;

pub use handlers::LinkPreviewDto;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route("/", post(handlers::preview)).with_state(state)
}
