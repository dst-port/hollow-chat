use std::sync::Arc;

use dashmap::DashMap;
use sqlx::PgPool;
use uuid::Uuid;

use crate::calls::PeerHandle;
use crate::devicelink::LinkPeer;
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
pub struct IceConfig {
    pub stun_urls: Arc<[String]>,
    pub turn_url: Option<Arc<str>>,
    pub turn_username: Option<Arc<str>>,
    pub turn_credential: Option<Arc<str>>,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub pepper: Arc<[u8]>,
    pub message_limiter: UserRateLimiter,
    pub attachments_dir: Arc<str>,
    pub http_client: reqwest::Client,
    pub billing: BillingConfig,
    pub ice: IceConfig,
    pub call_rooms: Arc<DashMap<Uuid, Vec<PeerHandle>>>,
    pub link_rooms: Arc<DashMap<Uuid, Vec<LinkPeer>>>,
}
