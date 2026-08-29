mod attachments;
mod auth;
mod badges;
mod billing;
mod blocks;
mod calls;
mod config;
mod db;
mod devicelink;
mod dms;
mod emoji;
mod error;
mod friends;
mod games;
mod gateway;
mod keys;
mod link_preview;
mod messages;
mod password;
mod permissions;
mod profile;
mod rate_limit;
mod reports;
mod roles;
mod secret_box;
mod sender_keys;
mod servers;
mod session;
mod social;
mod state;
mod telegram;
mod totp;

use axum::http::{HeaderValue, Method};
use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::{AllowOrigin, CorsLayer};

use dashmap::DashMap;

use config::Config;
use rate_limit::UserRateLimiter;
use games::GameCoverConfig;
use state::{AppState, BillingConfig, IceConfig};

const MESSAGE_LIMIT_PER_WINDOW: u32 = 15;
const MESSAGE_LIMIT_WINDOW: Duration = Duration::from_secs(10);

async fn health() -> &'static str {
    "ok"
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();

    let config = Config::from_env();
    telegram::init(config.telegram_bot_token.clone(), config.telegram_chat_id.clone());
    if config.telegram_bot_token.is_some() {
        telegram::notify("HollowChat server starting up");
    }

    // A panicking request handler only kills that one tokio task by
    // default - the server keeps running, but silently, unless something's
    // watching the logs. This turns "silently" into "pings Telegram".
    std::panic::set_hook(Box::new(|info| {
        tracing::error!("panic: {info}");
        telegram::notify(format!("HollowChat server panic:\n{info}"));
    }));

    let cors_origins: Vec<HeaderValue> = config
        .cors_allowed_origins
        .iter()
        .filter_map(|origin| {
            HeaderValue::from_str(origin)
                .inspect_err(|_| tracing::warn!("ignoring invalid CORS origin: {origin}"))
                .ok()
        })
        .collect();
    if cors_origins.is_empty() {
        tracing::warn!("no valid CORS_ALLOWED_ORIGINS configured - cross-origin requests will all be rejected");
    }
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(cors_origins))
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers(tower_http::cors::Any);

    let pool = db::connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    std::fs::create_dir_all(&config.attachments_dir)
        .expect("failed to create attachments directory");

    let bunny = match (
        config.bunny_storage_zone,
        config.bunny_storage_api_key,
        config.bunny_cdn_base_url,
    ) {
        (Some(storage_zone), Some(storage_api_key), Some(cdn_base_url)) => {
            tracing::info!("attachments will be stored on Bunny CDN ({cdn_base_url})");
            Some(state::BunnyConfig {
                storage_zone: Arc::from(storage_zone.into_boxed_str()),
                storage_api_key: Arc::from(storage_api_key.into_boxed_str()),
                storage_region_host: Arc::from(config.bunny_storage_region_host.into_boxed_str()),
                cdn_base_url: Arc::from(cdn_base_url.trim_end_matches('/').to_string().into_boxed_str()),
            })
        }
        (None, None, None) => None,
        _ => panic!(
            "BUNNY_STORAGE_ZONE, BUNNY_STORAGE_API_KEY and BUNNY_CDN_BASE_URL must be set together or not at all"
        ),
    };

    let http_client = reqwest::Client::new();

    if let Some(days) = config.attachment_retention_days {
        attachments::retention::spawn(
            pool.clone(),
            config.attachments_dir.clone(),
            bunny.clone(),
            http_client.clone(),
            days,
        );
        tracing::info!("attachment retention sweep enabled: {days} days");
    }

    let state = AppState {
        pool,
        pepper: Arc::from(config.pepper.into_boxed_slice()),
        message_limiter: UserRateLimiter::new(MESSAGE_LIMIT_PER_WINDOW, MESSAGE_LIMIT_WINDOW),
        attachments_dir: Arc::from(config.attachments_dir.into_boxed_str()),
        bunny,
        http_client,
        billing: BillingConfig {
            lava_api_key: config.lava_api_key.map(|s| Arc::from(s.into_boxed_str())),
            lava_offer_id: config.lava_offer_id.map(|s| Arc::from(s.into_boxed_str())),
            lava_webhook_secret: config
                .lava_webhook_secret
                .map(|s| Arc::from(s.into_boxed_str())),
            lava_currency: config.lava_currency.map(|s| Arc::from(s.into_boxed_str())),
            app_base_url: Arc::from(config.app_base_url.into_boxed_str()),
        },
        ice: IceConfig {
            stun_urls: Arc::from(config.ice_stun_urls.into_boxed_slice()),
            turn_url: config.ice_turn_url.map(|s| Arc::from(s.into_boxed_str())),
            turn_username: config.ice_turn_username.map(|s| Arc::from(s.into_boxed_str())),
            turn_credential: config.ice_turn_credential.map(|s| Arc::from(s.into_boxed_str())),
        },
        call_rooms: Arc::new(DashMap::new()),
        link_rooms: Arc::new(DashMap::new()),
        bundle_fetch_locks: Arc::new(DashMap::new()),
        link_preview_cache: Arc::new(DashMap::new()),
        gateway_sockets: Arc::new(DashMap::new()),
        game_covers: GameCoverConfig {
            steamgriddb_api_key: config.steamgriddb_api_key.map(|s| Arc::from(s.into_boxed_str())),
        },
        game_cover_cache: Arc::new(DashMap::new()),
    };

    let app = Router::new()
        .route("/health", get(health))
        .nest("/auth", auth::router(state.clone()))
        .nest("/emoji", emoji::router(state.clone()))
        .nest("/servers", servers::router(state.clone()))
        .nest("/servers", roles::router(state.clone()))
        .nest("/channels", messages::router(state.clone()))
        .nest("/channels", sender_keys::router(state.clone()))
        .nest("/friends", friends::router(state.clone()))
        .nest("/games", games::router(state.clone()))
        .nest("/dms", dms::router(state.clone()))
        .nest("/keys", keys::router(state.clone()))
        .nest("/badges", badges::router(state.clone()))
        .nest("/profile", profile::router(state.clone()))
        .nest("/files", attachments::router(state.clone()))
        .nest("/billing", billing::router(state.clone()))
        .nest("/blocks", blocks::router(state.clone()))
        .nest("/calls", calls::router(state.clone()))
        .nest("/devicelink", devicelink::router(state.clone()))
        .nest("/link-preview", link_preview::router(state.clone()))
        .nest("/gateway", gateway::router(state.clone()))
        .nest("/reports", reports::router(state.clone()))
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(&config.bind_addr)
        .await
        .expect("failed to bind");
    tracing::info!("hollowchat-server listening on {}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
