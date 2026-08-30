use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::Response;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc::unbounded_channel;
use uuid::Uuid;

use crate::auth::resolve_token;
use crate::error::AppError;
use crate::gateway::{push_to_users, GatewayEvent};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
enum ClientMsg {
    /// The user is typing in a channel or dm; fan out to the other people there.
    Typing { context: String, id: Uuid },
}

async fn recipients_for(
    state: &AppState,
    sender: Uuid,
    context: &str,
    id: Uuid,
) -> Vec<Uuid> {
    match context {
        "channel" => sqlx::query_scalar::<_, Uuid>(
            "SELECT sm.user_id FROM server_members sm \
             JOIN channels c ON c.server_id = sm.server_id \
             WHERE c.id = $1 AND sm.user_id <> $2 \
               AND EXISTS (SELECT 1 FROM server_members me \
                           JOIN channels mc ON mc.server_id = me.server_id \
                           WHERE mc.id = $1 AND me.user_id = $2)",
        )
        .bind(id)
        .bind(sender)
        .fetch_all(&state.pool)
        .await
        .unwrap_or_default(),
        "dm" => {
            let direct: Vec<Uuid> = sqlx::query_scalar(
                "SELECT CASE WHEN user_a = $2 THEN user_b ELSE user_a END \
                 FROM dm_channels WHERE id = $1 AND (user_a = $2 OR user_b = $2)",
            )
            .bind(id)
            .bind(sender)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default();
            if !direct.is_empty() {
                return direct;
            }
            sqlx::query_scalar(
                "SELECT user_id FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id <> $2 \
                 AND EXISTS (SELECT 1 FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id = $2)",
            )
            .bind(id)
            .bind(sender)
            .fetch_all(&state.pool)
            .await
            .unwrap_or_default()
        }
        _ => Vec::new(),
    }
}

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
    let username: String =
        sqlx::query_scalar("SELECT username FROM users WHERE id = $1")
            .bind(session.user_id)
            .fetch_one(&state.pool)
            .await
            .unwrap_or_default();
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session.user_id, username)))
}

async fn handle_socket(socket: WebSocket, state: AppState, user_id: Uuid, username: String) {
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

    let mut last_typing = std::time::Instant::now() - std::time::Duration::from_secs(10);
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Close(_) => break,
            Message::Text(text) => {
                let Ok(ClientMsg::Typing { context, id }) = serde_json::from_str(&text) else {
                    continue;
                };
                // one typing fan-out per 2s per socket is plenty
                if last_typing.elapsed() < std::time::Duration::from_secs(2) {
                    continue;
                }
                last_typing = std::time::Instant::now();
                let targets = recipients_for(&state, user_id, &context, id).await;
                if !targets.is_empty() {
                    push_to_users(
                        &state,
                        &targets,
                        &GatewayEvent::Typing {
                            context,
                            id,
                            user_id,
                            username: username.clone(),
                        },
                    );
                }
            }
            _ => {}
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
