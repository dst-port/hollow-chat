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

const MAX_ROOM_SIZE: usize = 8;

#[derive(Debug, Deserialize)]
pub struct JoinLinkQuery {
    pub token: String,
}

pub async fn join_link(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<JoinLinkQuery>,
) -> Result<Response, AppError> {
    let session = resolve_token(&state.pool, &query.token).await?;

    if state
        .link_rooms
        .get(&session.user_id)
        .map(|room| room.len())
        .unwrap_or(0)
        >= MAX_ROOM_SIZE
    {
        return Err(AppError::Unauthorized);
    }

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session.user_id)))
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
    Data { payload: String },
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

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<Message>();
    let conn_id = Uuid::new_v4();

    state
        .link_rooms
        .entry(user_id)
        .or_default()
        .push(LinkPeer { conn_id, sender: tx.clone() });

    broadcast_except(&state, user_id, conn_id, &ServerMsg::PeerJoined);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

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
                broadcast_except(&state, user_id, conn_id, &ServerMsg::Data { payload });
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
