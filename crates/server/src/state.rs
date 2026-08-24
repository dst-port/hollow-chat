use std::sync::Arc;

use sqlx::PgPool;

use crate::rate_limit::UserRateLimiter;

#[derive(Clone)]
pub struct BillingConfig {
    pub lava_api_key: Option<Arc<str>>,
    pub lava_offer_id: Option<Arc<str>>,
    pub lava_webhook_secret: Option<Arc<str>>,
    pub lava_currency: Option<Arc<str>>,
    pub app_base_url: Arc<str>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub pepper: Arc<[u8]>,
    pub message_limiter: UserRateLimiter,
    pub attachments_dir: Arc<str>,
    pub http_client: reqwest::Client,
    pub billing: BillingConfig,
}
