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
    /// "Something in this scope changed for you - refetch it." The client
    /// keeps the DB as the source of truth and re-pulls the relevant list
    /// ("servers" | "friends" | "dms") rather than us shipping granular
    /// diffs. Sent to every affected user (including, harmlessly, the one
    /// who made the change - their refetch just reconciles to the same
    /// state). Also replayed to a socket right after it reconnects so a
    /// client that was offline catches up.
    Sync {
        scope: String,
    },
    /// A new message landed in a channel or DM the recipient belongs to.
    /// Carries the fully-assembled message DTO (still E2E-encrypted blob) so
    /// the client renders it with no follow-up fetch. `context` is
    /// "channel" | "dm"; `channel_id` is the channel/dm-channel id.
    MessageCreated {
        context: String,
        channel_id: Uuid,
        message: serde_json::Value,
    },
    /// An existing message changed - edit, reaction, or pin. The client
    /// refetches just that message (its reacted/own-view fields are
    /// per-recipient, so we can't broadcast one shared copy).
    MessageUpdated {
        context: String,
        channel_id: Uuid,
        message_id: Uuid,
    },
    MessageDeleted {
        context: String,
        channel_id: Uuid,
        message_id: Uuid,
    },
    Typing {
        /// "channel" or "dm" — where the person is typing.
        context: String,
        /// channel id or dm channel id.
        id: Uuid,
        user_id: Uuid,
        username: String,
    },
    PresenceUpdate {
        user_id: Uuid,
        presence: String,
        status_text: Option<String>,
        activity_application: Option<String>,
        activity_details: Option<String>,
        activity_state: Option<String>,
        activity_image: Option<String>,
        activity_small_image: Option<String>,
        activity_small_text: Option<String>,
        activity_started_at: Option<DateTime<Utc>>,
        activity_party_size: Option<i16>,
        activity_party_max: Option<i16>,
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

pub fn push_to_users(state: &AppState, user_ids: &[Uuid], event: &GatewayEvent) {
    let Ok(text) = serde_json::to_string(event) else { return };
    for uid in user_ids {
        if let Some(sockets) = state.gateway_sockets.get(uid) {
            for sender in sockets.iter() {
                let _ = sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

/// Tell these users to refetch a scope ("servers" | "friends" | "dms").
/// Safe to over-notify: a client that isn't showing that list ignores it,
/// and the notifier itself just reconciles to the same state.
pub fn notify_sync(state: &AppState, user_ids: &[Uuid], scope: &str) {
    if user_ids.is_empty() {
        return;
    }
    push_to_users(
        state,
        user_ids,
        &GatewayEvent::Sync {
            scope: scope.to_string(),
        },
    );
}

/// Member user-ids of a server - the fan-out target for guild-tree changes.
pub async fn server_member_ids(pool: &sqlx::PgPool, server_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar::<_, Uuid>("SELECT user_id FROM server_members WHERE server_id = $1")
        .bind(server_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

/// User-ids that can see a server channel (every member of its server).
pub async fn channel_member_ids(pool: &sqlx::PgPool, channel_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT sm.user_id FROM server_members sm \
         JOIN channels c ON c.server_id = sm.server_id \
         WHERE c.id = $1",
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

/// Member user-ids of a DM/group-DM channel.
pub async fn dm_member_ids(pool: &sqlx::PgPool, dm_channel_id: Uuid) -> Vec<Uuid> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT user_id FROM dm_channel_members WHERE dm_channel_id = $1",
    )
    .bind(dm_channel_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::connect))
        .with_state(state)
}
