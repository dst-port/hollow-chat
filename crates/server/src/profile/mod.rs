mod handlers;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", patch(handlers::update_profile))
        .route("/presence", put(handlers::set_presence))
        .route("/activity", put(handlers::set_activity))
        .route("/avatar", put(handlers::set_avatar).delete(handlers::clear_avatar))
        .route("/banner", put(handlers::set_banner).delete(handlers::clear_banner))
        .route("/connections", post(handlers::add_connection))
        .route("/connections/{connection_id}", delete(handlers::remove_connection))
        .route("/widgets", post(handlers::add_widget))
        .route(
            "/widgets/{widget_id}",
            patch(handlers::update_widget).delete(handlers::remove_widget),
        )
        .route("/{username}", get(handlers::get_profile))
        .route("/{username}/connections", get(handlers::list_connections))
        .route("/{username}/widgets", get(handlers::list_widgets))
        .with_state(state)
}
