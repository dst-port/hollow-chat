mod attachments;
mod auth;
mod billing;
mod blocks;
mod config;
mod db;
mod dms;
mod emoji;
mod error;
mod friends;
mod keys;
mod messages;
mod password;
mod permissions;
mod rate_limit;
mod roles;
mod servers;
mod session;
mod social;
mod state;

use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower_http::cors::CorsLayer;

use config::Config;
use rate_limit::UserRateLimiter;
use state::{AppState, BillingConfig};

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

    let pool = db::connect(&config.database_url)
        .await
        .expect("failed to connect to database");

    sqlx::migrate!("../../migrations")
        .run(&pool)
        .await
        .expect("failed to run migrations");

    std::fs::create_dir_all(&config.attachments_dir)
        .expect("failed to create attachments directory");

    let state = AppState {
        pool,
        pepper: Arc::from(config.pepper.into_boxed_slice()),
        message_limiter: UserRateLimiter::new(MESSAGE_LIMIT_PER_WINDOW, MESSAGE_LIMIT_WINDOW),
        attachments_dir: Arc::from(config.attachments_dir.into_boxed_str()),
        http_client: reqwest::Client::new(),
        billing: BillingConfig {
            lava_api_key: config.lava_api_key.map(|s| Arc::from(s.into_boxed_str())),
            lava_offer_id: config.lava_offer_id.map(|s| Arc::from(s.into_boxed_str())),
            lava_webhook_secret: config
                .lava_webhook_secret
                .map(|s| Arc::from(s.into_boxed_str())),
            lava_currency: config.lava_currency.map(|s| Arc::from(s.into_boxed_str())),
            app_base_url: Arc::from(config.app_base_url.into_boxed_str()),
        },
    };

    let app = Router::new()
        .route("/health", get(health))
        .nest("/auth", auth::router(state.clone()))
        .nest("/emoji", emoji::router(state.clone()))
        .nest("/servers", servers::router(state.clone()))
        .nest("/servers", roles::router(state.clone()))
        .nest("/channels", messages::router(state.clone()))
        .nest("/friends", friends::router(state.clone()))
        .nest("/dms", dms::router(state.clone()))
        .nest("/keys", keys::router(state.clone()))
        .nest("/files", attachments::router(state.clone()))
        .nest("/billing", billing::router(state.clone()))
        .nest("/blocks", blocks::router(state.clone()))
        .with_state(state)
        .layer(CorsLayer::permissive());

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
