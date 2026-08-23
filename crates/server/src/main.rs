mod auth;
mod config;
mod db;
mod error;
mod password;
mod rate_limit;
mod session;
mod state;

use axum::routing::get;
use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;

use config::Config;
use state::AppState;

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

    let state = AppState {
        pool,
        pepper: Arc::from(config.pepper.into_boxed_slice()),
    };

    let app = Router::new()
        .route("/health", get(health))
        .nest("/auth", auth::router(state.clone()))
        .with_state(state);

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
