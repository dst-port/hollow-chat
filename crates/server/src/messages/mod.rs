mod handlers;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{channel_id}/messages", get(handlers::list_messages))
        .route("/{channel_id}/messages", post(handlers::send_message))
        .route("/{channel_id}/pinned", get(handlers::list_pinned))
        .route("/{channel_id}/messages/{message_id}", patch(handlers::edit_message))
        .route("/{channel_id}/messages/{message_id}", delete(handlers::delete_message))
        .route("/{channel_id}/messages/{message_id}/pin", put(handlers::pin_message))
        .route("/{channel_id}/messages/{message_id}/pin", delete(handlers::unpin_message))
        .route(
            "/{channel_id}/messages/{message_id}/reactions/{emoji}",
            put(handlers::add_reaction),
        )
        .route(
            "/{channel_id}/messages/{message_id}/reactions/{emoji}",
            delete(handlers::remove_reaction),
        )
        .with_state(state)
}
