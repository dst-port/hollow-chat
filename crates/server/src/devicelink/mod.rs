mod handlers;

use axum::extract::ws::Message;
use axum::routing::get;
use axum::Router;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::state::AppState;

#[derive(Clone)]
pub struct LinkPeer {
    pub conn_id: Uuid,
    pub sender: UnboundedSender<Message>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::join_link))
        .with_state(state)
}
