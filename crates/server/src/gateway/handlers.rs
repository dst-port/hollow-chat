use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc::unbounded_channel;

use crate::auth::resolve_token;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ConnectQuery {
    pub token: String,
}

pub async fn connect(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(query): Query<ConnectQuery>,
) -> Result<Response, AppError> {
    let session = resolve_token(&state.pool, &query.token).await?;
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session.user_id)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: uuid::Uuid) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let (tx, mut rx) = unbounded_channel::<Message>();

    state.gateway_sockets.entry(user_id).or_default().push(tx);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_tx.send(msg).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(msg)) = ws_rx.next().await {
        if matches!(msg, Message::Close(_)) {
            break;
        }
    }

    send_task.abort();

    if let Some(mut sockets) = state.gateway_sockets.get_mut(&user_id) {
        sockets.retain(|s| !s.is_closed());
    }
    if state.gateway_sockets.get(&user_id).map(|s| s.is_empty()).unwrap_or(false) {
        state.gateway_sockets.remove(&user_id);
    }
}
