use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Path, Query, State};
use axum::response::Response;
use axum::Json;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;

use crate::auth::{resolve_token, AuthSession};
use crate::error::AppError;
use crate::state::AppState;

use super::PeerHandle;

#[derive(Debug, Serialize)]
pub struct IceServerDto {
    pub urls: Vec<String>,
    pub username: Option<String>,
    pub credential: Option<String>,
}

pub async fn ice_servers(
    State(state): State<AppState>,
    _session: AuthSession,
) -> Json<Vec<IceServerDto>> {
    let mut servers = vec![IceServerDto {
        urls: state.ice.stun_urls.to_vec(),
        username: None,
        credential: None,
    }];

    if let Some(turn_url) = &state.ice.turn_url {
        servers.push(IceServerDto {
            urls: vec![turn_url.to_string()],
            username: state.ice.turn_username.as_ref().map(|s| s.to_string()),
            credential: state.ice.turn_credential.as_ref().map(|s| s.to_string()),
        });
    }

    Json(servers)
}

async fn authorize_room(pool: &sqlx::PgPool, room_id: Uuid, user_id: Uuid) -> Result<(), AppError> {
    let channel: Option<(Uuid,)> = sqlx::query_as("SELECT server_id FROM channels WHERE id = $1")
        .bind(room_id)
        .fetch_optional(pool)
        .await?;

    if let Some((server_id,)) = channel {
        let member: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM server_members WHERE server_id = $1 AND user_id = $2",
        )
        .bind(server_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;
        return member.map(|_| ()).ok_or(AppError::Unauthorized);
    }

    let dm: Option<(Uuid, Uuid)> = sqlx::query_as("SELECT user_a, user_b FROM dm_channels WHERE id = $1")
        .bind(room_id)
        .fetch_optional(pool)
        .await?;

    match dm {
        Some((a, b)) if a == user_id || b == user_id => Ok(()),
        Some(_) => Err(AppError::Unauthorized),
        None => Err(AppError::NotFound),
    }
}

#[derive(Debug, Deserialize)]
pub struct JoinCallQuery {
    pub token: String,
}

pub async fn join_call(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(room_id): Path<Uuid>,
    Query(query): Query<JoinCallQuery>,
) -> Result<Response, AppError> {
    let session = resolve_token(&state.pool, &query.token).await?;
    authorize_room(&state.pool, room_id, session.user_id).await?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, room_id, session)))
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerInfo {
    pub user_id: Uuid,
    pub username: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ClientMsg {
    Offer { to: Uuid, sdp: String },
    Answer { to: Uuid, sdp: String },
    IceCandidate { to: Uuid, candidate: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ServerMsg {
    RoomState { members: Vec<PeerInfo> },
    PeerJoined { user_id: Uuid, username: String },
    PeerLeft { user_id: Uuid },
    Offer { from: Uuid, from_username: String, sdp: String },
    Answer { from: Uuid, sdp: String },
    IceCandidate { from: Uuid, candidate: String },
}

fn send_to(state: &AppState, room_id: Uuid, target: Uuid, msg: &ServerMsg) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    if let Some(room) = state.call_rooms.get(&room_id) {
        for peer in room.iter() {
            if peer.user_id == target {
                let _ = peer.sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

fn broadcast_except(state: &AppState, room_id: Uuid, except: Uuid, msg: &ServerMsg) {
    let Ok(text) = serde_json::to_string(msg) else { return };
    if let Some(room) = state.call_rooms.get(&room_id) {
        for peer in room.iter() {
            if peer.user_id != except {
                let _ = peer.sender.send(Message::Text(text.clone().into()));
            }
        }
    }
}

async fn handle_socket(socket: WebSocket, state: AppState, room_id: Uuid, session: AuthSession) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<Message>();

    let existing: Vec<PeerInfo> = state
        .call_rooms
        .get(&room_id)
        .map(|room| {
            room.iter()
                .map(|p| PeerInfo { user_id: p.user_id, username: p.username.clone() })
                .collect()
        })
        .unwrap_or_default();

    state.call_rooms.entry(room_id).or_default().push(PeerHandle {
        user_id: session.user_id,
        username: session.username.clone(),
        sender: tx.clone(),
    });

    if let Ok(text) = serde_json::to_string(&ServerMsg::RoomState { members: existing }) {
        let _ = tx.send(Message::Text(text.into()));
    }

    broadcast_except(
        &state,
        room_id,
        session.user_id,
        &ServerMsg::PeerJoined { user_id: session.user_id, username: session.username.clone() },
    );

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

        let server_msg = match client_msg {
            ClientMsg::Offer { to, sdp } => (
                to,
                ServerMsg::Offer { from: session.user_id, from_username: session.username.clone(), sdp },
            ),
            ClientMsg::Answer { to, sdp } => (to, ServerMsg::Answer { from: session.user_id, sdp }),
            ClientMsg::IceCandidate { to, candidate } => {
                (to, ServerMsg::IceCandidate { from: session.user_id, candidate })
            }
        };

        send_to(&state, room_id, server_msg.0, &server_msg.1);
    }

    send_task.abort();

    if let Some(mut room) = state.call_rooms.get_mut(&room_id) {
        room.retain(|p| p.user_id != session.user_id);
    }
    if state.call_rooms.get(&room_id).map(|r| r.is_empty()).unwrap_or(false) {
        state.call_rooms.remove(&room_id);
    }

    broadcast_except(&state, room_id, session.user_id, &ServerMsg::PeerLeft { user_id: session.user_id });
}
