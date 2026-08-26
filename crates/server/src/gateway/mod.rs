mod handlers;

use axum::extract::ws::Message;
use axum::routing::get;
use axum::Router;
use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::social::friend_ids;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum GatewayEvent {
    PresenceUpdate {
        user_id: Uuid,
        presence: String,
        status_text: Option<String>,
        activity_application: Option<String>,
        activity_details: Option<String>,
        activity_state: Option<String>,
        activity_image: Option<String>,
        activity_started_at: Option<DateTime<Utc>>,
        media_application: Option<String>,
        media_details: Option<String>,
        media_state: Option<String>,
    },
}

pub async fn push_to_friends(state: &AppState, user_id: Uuid, event: &GatewayEvent) {
    let Ok(text) = serde_json::to_string(event) else { return };
    let Ok(friends) = friend_ids(&state.pool, user_id).await else { return };
    for friend_id in friends {
        if let Some(sockets) = state.gateway_sockets.get(&friend_id) {
            for sender in sockets.iter() {
                let _ = sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::connect))
        .with_state(state)
}
