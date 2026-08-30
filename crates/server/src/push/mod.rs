//! Web Push notifications for when a user has no gateway socket open (app
//! closed / backgrounded). Browsers register a `PushSubscription`; we store it
//! and, on a DM or an @mention the recipient is offline for, encrypt a small
//! JSON payload and hand it to their push service.

mod webpush;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

pub use webpush::VapidKey;

/// Parsed VAPID config held on `AppState` (None disables push entirely).
pub struct Vapid {
    pub key: VapidKey,
    pub subject: String,
}

impl Vapid {
    pub fn from_env() -> Option<Self> {
        let pem_b64 = std::env::var("VAPID_PRIVATE_KEY_B64").ok()?;
        let pem = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            pem_b64.trim(),
        )
        .ok()?;
        let pem = String::from_utf8(pem).ok()?;
        let key = match VapidKey::from_sec1_pem(&pem) {
            Ok(k) => k,
            Err(e) => {
                tracing::error!("VAPID key parse failed: {e}");
                return None;
            }
        };
        let subject = std::env::var("VAPID_SUBJECT")
            .unwrap_or_else(|_| "mailto:admin@hollowchat.org".to_string());
        Some(Self { key, subject })
    }
}

/// JSON the service worker receives in its `push` event.
#[derive(Debug, Serialize)]
pub struct PushPayload {
    pub title: String,
    pub body: String,
    /// Where a click should take the user (currently just the app root).
    pub url: String,
    /// Collapses repeat notifications for the same conversation.
    pub tag: String,
}

#[derive(sqlx::FromRow)]
struct StoredSub {
    endpoint: String,
    p256dh: String,
    auth: String,
}

/// Send `payload` to every one of these users who currently has NO gateway
/// socket connected. Fire-and-forget: the push round-trips happen on a
/// detached task so they never delay the caller's HTTP response.
pub fn notify_offline(state: &AppState, user_ids: &[Uuid], payload: PushPayload) {
    let Some(vapid) = state.vapid.clone() else {
        return;
    };
    let offline: Vec<Uuid> = user_ids
        .iter()
        .copied()
        .filter(|u| {
            state
                .gateway_sockets
                .get(u)
                .map(|s| s.is_empty())
                .unwrap_or(true)
        })
        .collect();
    if offline.is_empty() {
        return;
    }
    let Ok(body) = serde_json::to_vec(&payload) else {
        return;
    };
    let pool = state.pool.clone();
    let http = state.http_client.clone();
    tokio::spawn(async move {
        let subs: Vec<StoredSub> = sqlx::query_as(
            "SELECT endpoint, p256dh, auth FROM push_subscriptions WHERE user_id = ANY($1)",
        )
        .bind(&offline)
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

        for sub in subs {
            match webpush::deliver(
                &http,
                &vapid.key,
                &vapid.subject,
                &sub.endpoint,
                &sub.p256dh,
                &sub.auth,
                &body,
            )
            .await
            {
                Ok(()) => {}
                Err(webpush::DeliveryError::Gone) => {
                    let _ = sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1")
                        .bind(&sub.endpoint)
                        .execute(&pool)
                        .await;
                }
                Err(webpush::DeliveryError::Transient(e)) => {
                    tracing::debug!("web push to {} failed: {e}", sub.endpoint);
                }
            }
        }
    });
}

#[derive(Deserialize)]
struct SubscribeBody {
    endpoint: String,
    keys: SubscribeKeys,
}

#[derive(Deserialize)]
struct SubscribeKeys {
    p256dh: String,
    auth: String,
}

async fn subscribe(
    State(state): State<AppState>,
    session: AuthSession,
    Json(body): Json<SubscribeBody>,
) -> Result<(), AppError> {
    if body.endpoint.is_empty() || body.keys.p256dh.is_empty() || body.keys.auth.is_empty() {
        return Err(AppError::BadRequest("incomplete subscription".into()));
    }
    sqlx::query(
        "INSERT INTO push_subscriptions (endpoint, user_id, p256dh, auth) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (endpoint) DO UPDATE \
           SET user_id = EXCLUDED.user_id, p256dh = EXCLUDED.p256dh, auth = EXCLUDED.auth",
    )
    .bind(&body.endpoint)
    .bind(session.user_id)
    .bind(&body.keys.p256dh)
    .bind(&body.keys.auth)
    .execute(&state.pool)
    .await?;
    Ok(())
}

#[derive(Deserialize)]
struct UnsubscribeBody {
    endpoint: String,
}

async fn unsubscribe(
    State(state): State<AppState>,
    session: AuthSession,
    Json(body): Json<UnsubscribeBody>,
) -> Result<(), AppError> {
    sqlx::query("DELETE FROM push_subscriptions WHERE endpoint = $1 AND user_id = $2")
        .bind(&body.endpoint)
        .bind(session.user_id)
        .execute(&state.pool)
        .await?;
    Ok(())
}

#[derive(Serialize)]
struct VapidKeyResponse {
    key: Option<String>,
}

async fn vapid_key(State(state): State<AppState>) -> Json<VapidKeyResponse> {
    Json(VapidKeyResponse {
        key: state.vapid.as_ref().map(|v| v.key.public_b64.clone()),
    })
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/vapid", get(vapid_key))
        .route("/subscribe", post(subscribe))
        .route("/unsubscribe", post(unsubscribe))
        .with_state(state)
}
