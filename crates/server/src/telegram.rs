// Server-health alerts to Telegram - not user-facing notifications, just
// "something on the backend broke" pings for whoever's operating this
// instance. Fire-and-forget: never on the hot path of a request, never
// something a failure here should be allowed to affect.

use std::sync::OnceLock;

struct TelegramConfig {
    token: String,
    chat_id: String,
    client: reqwest::Client,
}

static CONFIG: OnceLock<Option<TelegramConfig>> = OnceLock::new();

pub fn init(token: Option<String>, chat_id: Option<String>) {
    let config = match (token, chat_id) {
        (Some(token), Some(chat_id)) => Some(TelegramConfig {
            token,
            chat_id,
            client: reqwest::Client::new(),
        }),
        _ => None,
    };
    let _ = CONFIG.set(config);
}

/// Sends `message` to the configured chat in the background. No-ops
/// silently if Telegram alerting was never configured (most self-hosted
/// instances) or if the request itself fails - this must never be allowed
/// to cascade into more errors.
pub fn notify(message: impl Into<String>) {
    let Some(Some(config)) = CONFIG.get() else { return };
    let message = message.into();
    let token = config.token.clone();
    let chat_id = config.chat_id.clone();
    let client = config.client.clone();

    tokio::spawn(async move {
        let url = format!("https://api.telegram.org/bot{token}/sendMessage");
        let _ = client
            .post(url)
            .json(&serde_json::json!({
                "chat_id": chat_id,
                "text": message,
                "disable_web_page_preview": true
            }))
            .send()
            .await;
    });
}
