pub mod handlers;

use axum::routing::{delete, patch, post, put};
use axum::Router;

use crate::state::AppState;

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/{server_id}/roles", axum::routing::get(handlers::list_roles))
        .route("/{server_id}/roles", post(handlers::create_role))
        .route("/{server_id}/roles/{role_id}", patch(handlers::update_role))
        .route("/{server_id}/roles/{role_id}", delete(handlers::delete_role))
        .route(
            "/{server_id}/members/{user_id}/roles/{role_id}",
            put(handlers::assign_role),
        )
        .route(
            "/{server_id}/members/{user_id}/roles/{role_id}",
            delete(handlers::unassign_role),
        )
        .with_state(state)
}
