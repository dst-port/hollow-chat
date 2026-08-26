mod handlers;

use axum::routing::{delete, get, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", post(handlers::create_server))
        .route("/", get(handlers::list_servers))
        .route("/{id}", patch(handlers::rename_server))
        .route("/{id}/icon", put(handlers::set_server_icon))
        .route("/{id}/icon", delete(handlers::clear_server_icon))
        .route("/{id}/leave", delete(handlers::leave_server))
        .route("/{id}/channels", post(handlers::create_channel))
        .route(
            "/{id}/channels/{channel_id}/slowmode",
            patch(handlers::set_slowmode),
        )
        .route("/{id}/members", get(handlers::list_members))
        .route("/{id}/members/{user_id}/kick", post(handlers::kick_member))
        .route("/{id}/bans", get(handlers::list_bans))
        .route("/{id}/bans/{user_id}", put(handlers::ban_member))
        .route("/{id}/bans/{user_id}", delete(handlers::unban_member))
        .route("/{id}/invite", get(handlers::get_invite))
        .route("/{id}/invite", put(handlers::set_vanity_invite))
        .route("/join", post(handlers::join_server))
        .route("/{id}/boosts", get(handlers::get_boosts))
        .route("/{id}/boosts", post(handlers::add_boost))
        .route("/{id}/boosts", delete(handlers::remove_boost))
        .route("/{id}/emoji", get(handlers::list_custom_emoji))
        .route("/{id}/emoji", post(handlers::add_custom_emoji))
        .route("/{id}/emoji/{emoji_id}", delete(handlers::remove_custom_emoji))
        .with_state(state)
}
