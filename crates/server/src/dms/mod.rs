mod handlers;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::list_dms))
        .route("/", post(handlers::open_dm))
        .route("/{id}/messages", get(handlers::list_messages))
        .route("/{id}/messages", post(handlers::send_message))
        .route("/{id}/pinned", get(handlers::list_pinned))
        .route("/{id}/messages/{message_id}", patch(handlers::edit_message))
        .route("/{id}/messages/{message_id}", delete(handlers::delete_message))
        .route("/{id}/messages/{message_id}/pin", put(handlers::pin_message))
        .route("/{id}/messages/{message_id}/pin", delete(handlers::unpin_message))
        .route(
            "/{id}/messages/{message_id}/reactions/{emoji}",
            put(handlers::add_reaction),
        )
        .route(
            "/{id}/messages/{message_id}/reactions/{emoji}",
            delete(handlers::remove_reaction),
        )
        .with_state(state)
}
