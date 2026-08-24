mod handlers;

use axum::extract::ws::Message;
use axum::routing::get;
use axum::Router;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Clone)]
pub struct PeerHandle {
    pub user_id: Uuid,
    pub username: String,
    pub sender: UnboundedSender<Message>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/ice-servers", get(handlers::ice_servers))
        .route("/{room_id}", get(handlers::join_call))
        .with_state(state)
}
