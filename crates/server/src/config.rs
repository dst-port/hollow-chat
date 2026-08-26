pub struct Config {
    pub database_url: String,
    pub pepper: Vec<u8>,
    pub bind_addr: String,
    pub attachments_dir: String,
    pub lava_api_key: Option<String>,
    pub lava_offer_id: Option<String>,
    pub lava_webhook_secret: Option<String>,
    pub lava_currency: Option<String>,
    pub app_base_url: String,
    pub ice_stun_urls: Vec<String>,
    pub ice_turn_url: Option<String>,
    pub ice_turn_username: Option<String>,
    pub ice_turn_credential: Option<String>,
    pub steamgriddb_api_key: Option<String>,
    pub cors_allowed_origins: Vec<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        let database_url =
            std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pepper = std::env::var("PASSWORD_PEPPER")
            .expect("PASSWORD_PEPPER must be set")
            .into_bytes();
        let bind_addr =
            std::env::var("BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());
        let attachments_dir =
            std::env::var("ATTACHMENTS_DIR").unwrap_or_else(|_| "./data/attachments".to_string());
        let lava_api_key = std::env::var("LAVA_API_KEY").ok();
        let lava_offer_id = std::env::var("LAVA_OFFER_ID").ok();
        let lava_webhook_secret = std::env::var("LAVA_WEBHOOK_SECRET").ok();
        let lava_currency = std::env::var("LAVA_CURRENCY").ok();
        // Matches tauri.conf.json's devUrl exactly ("localhost", not
        // "127.0.0.1" - different origins for CORS purposes even though
        // they resolve to the same address) so a fresh dev checkout works
        // without anyone needing to discover this the hard way.
        let app_base_url =
            std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://localhost:1420".to_string());
        let ice_stun_urls = std::env::var("ICE_STUN_URLS")
            .unwrap_or_else(|_| "stun:stun.l.google.com:19302".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let ice_turn_url = std::env::var("ICE_TURN_URL").ok();
        let ice_turn_username = std::env::var("ICE_TURN_USERNAME").ok();
        let ice_turn_credential = std::env::var("ICE_TURN_CREDENTIAL").ok();
        let steamgriddb_api_key = std::env::var("STEAMGRIDDB_API_KEY").ok();
        // No default beyond APP_BASE_URL: an empty/unset allowlist is safer
        // than accidentally permissive, and self-hosters running a web or
        // browser-extension client alongside the desktop app need to opt
        // those origins in explicitly.
        let cors_allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .ok()
            .map(|raw| {
                raw.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| vec![app_base_url.clone()]);
        let telegram_bot_token = std::env::var("TELEGRAM_BOT_TOKEN").ok();
        let telegram_chat_id = std::env::var("TELEGRAM_CHAT_ID").ok();

        Self {
            database_url,
            pepper,
            bind_addr,
            attachments_dir,
            lava_api_key,
            lava_offer_id,
            lava_webhook_secret,
            lava_currency,
            app_base_url,
            ice_stun_urls,
            ice_turn_url,
            ice_turn_username,
            ice_turn_credential,
            steamgriddb_api_key,
            cors_allowed_origins,
            telegram_bot_token,
            telegram_chat_id,
        }
    }
}
