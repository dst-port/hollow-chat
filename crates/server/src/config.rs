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
        let app_base_url =
            std::env::var("APP_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:1420".to_string());

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
        }
    }
}
