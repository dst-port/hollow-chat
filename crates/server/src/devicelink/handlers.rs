use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;

use crate::auth::resolve_token;
use crate::error::AppError;
use crate::state::AppState;

use super::LinkPeer;

const FREE_ROOM_SIZE: usize = 3;
const PREMIUM_ROOM_SIZE: usize = 8;
const MAX_BACKLOG: i64 = 500;
const BACKLOG_RETENTION_DAYS: i64 = 30;

#[derive(Debug, Deserialize)]
pub struct JoinLinkQuery {
    pub token: String,
    #[serde(default)]
    pub since: i64,
}

pub async fn join_link(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<JoinLinkQuery>,
) -> Result<Response, AppError> {
    let session = resolve_token(&state.pool, &query.token).await?;

    let max_room_size = if crate::billing::is_premium(&state.pool, session.user_id).await? {
        PREMIUM_ROOM_SIZE
    } else {
        FREE_ROOM_SIZE
    };

    if state
        .link_rooms
        .get(&session.user_id)
        .map(|room| room.len())
        .unwrap_or(0)
        >= max_room_size
    {
        return Err(AppError::Unauthorized);
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session.user_id, query.since)))
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ClientMsg {
    Data { payload: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ServerMsg {
    PeerJoined,
    PeerLeft,
    Data { id: i64, payload: String },
}

fn broadcast_except(state: &AppState, user_id: Uuid, except: Uuid, msg: &ServerMsg) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    if let Some(room) = state.link_rooms.get(&user_id) {
        for peer in room.iter() {
            if peer.conn_id != except {
                let _ = peer.sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

/// Persists a delta so a device that's offline right now (not just briefly
/// unreachable, but genuinely disconnected when this was sent) still gets
/// it once it reconnects - the live relay alone only reaches devices that
/// happen to be online at that exact moment. The payload is the same
/// end-to-end-encrypted envelope already sent over the wire, sealed with
/// the pairing's sync key that the server never has - storing it here is
/// no different, trust-wise, than storing message ciphertext.
async fn enqueue(state: &AppState, user_id: Uuid, payload: &str) -> Option<i64> {
    let row: Option<(i64,)> = sqlx::query_as(
        "INSERT INTO devicelink_deltas (user_id, payload) VALUES ($1, $2) RETURNING id",
    )
    .bind(user_id)
    .bind(payload)
    .fetch_optional(&state.pool)
    .await
    .ok()
    .flatten();

    let _ = sqlx::query(
        "DELETE FROM devicelink_deltas WHERE user_id = $1 AND created_at < now() - make_interval(days => $2)",
    )
    .bind(user_id)
    .bind(BACKLOG_RETENTION_DAYS as i32)
    .execute(&state.pool)
    .await;

    row.map(|(id,)| id)
}

async fn backlog_since(state: &AppState, user_id: Uuid, since: i64) -> Vec<(i64, String)> {
    sqlx::query_as(
        "SELECT id, payload FROM devicelink_deltas \
         WHERE user_id = $1 AND id > $2 ORDER BY id ASC LIMIT $3",
    )
    .bind(user_id)
    .bind(since)
    .bind(MAX_BACKLOG)
    .fetch_all(&state.pool)
    .await
    .unwrap_or_default()
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, since: i64) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<Message>();
    let conn_id = Uuid::new_v4();

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    // Queued before this connection is registered in the room, so no live
    // broadcast from another device can race ahead of the backlog and
    // arrive out of order.
    for (id, payload) in backlog_since(&state, user_id, since).await {
        let Ok(text) = serde_json::to_string(&ServerMsg::Data { id, payload }) else { continue };
        let _ = tx.send(Message::Text(text.into()));
    }

    state
        .link_rooms
        .entry(user_id)
        .or_default()
        .push(LinkPeer { conn_id, sender: tx.clone() });

    broadcast_except(&state, user_id, conn_id, &ServerMsg::PeerJoined);

    while let Some(Ok(msg)) = ws_rx.next().await {
        let Message::Text(text) = msg else {
            if matches!(msg, Message::Close(_)) {
                break;
            }
            continue;
        };

        let Ok(client_msg) = serde_json::from_str::<ClientMsg>(&text) else { continue };
        match client_msg {
            ClientMsg::Data { payload } => {
                if let Some(id) = enqueue(&state, user_id, &payload).await {
                    broadcast_except(&state, user_id, conn_id, &ServerMsg::Data { id, payload });
                }
            }
        }
    }

    send_task.abort();

    if let Some(mut room) = state.link_rooms.get_mut(&user_id) {
        room.retain(|p| p.conn_id != conn_id);
    }
    if state
        .link_rooms
        .get(&user_id)
        .map(|r| r.is_empty())
        .unwrap_or(false)
    {
        state.link_rooms.remove(&user_id);
    }

    broadcast_except(&state, user_id, conn_id, &ServerMsg::PeerLeft);
}
