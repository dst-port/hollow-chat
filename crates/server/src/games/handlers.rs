use std::time::{Duration, Instant};

use axum::extract::{Query, State};
use axum::Json;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use serde::{Deserialize, Serialize};

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

/// Covers barely ever change - a week-long cache keeps us from re-querying
/// SteamGridDB every time a friend's Rich Presence updates for the same
/// game.
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub(crate) fn spawn_cache_pruner(state: &AppState) {
    let cache = state.game_cover_cache.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CACHE_TTL);
        loop {
            interval.tick().await;
            cache.retain(|_, (inserted, _)| inserted.elapsed() < CACHE_TTL);
        }
    });
}

#[derive(Debug, Deserialize)]
pub struct CoverQuery {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct CoverDto {
    pub url: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SearchResult {
    data: Vec<SearchEntry>,
}

#[derive(Debug, Deserialize)]
struct SearchEntry {
    id: u64,
}

#[derive(Debug, Deserialize)]
struct AssetResult {
    data: Vec<AssetEntry>,
}

#[derive(Debug, Deserialize)]
struct AssetEntry {
    url: String,
}

async fn fetch_game_id(state: &AppState, api_key: &str, name: &str) -> Option<u64> {
    let encoded = utf8_percent_encode(name, NON_ALPHANUMERIC);
    let url = format!("https://www.steamgriddb.com/api/v2/search/autocomplete/{encoded}");
    let res = state
        .http_client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .ok()?;
    let body: SearchResult = res.json().await.ok()?;
    body.data.first().map(|entry| entry.id)
}

async fn fetch_asset(state: &AppState, api_key: &str, kind: &str, game_id: u64) -> Option<String> {
    let url = format!("https://www.steamgriddb.com/api/v2/{kind}/game/{game_id}");
    let res = state
        .http_client
        .get(&url)
        .bearer_auth(api_key)
        .send()
        .await
        .ok()?;
    let body: AssetResult = res.json().await.ok()?;
    body.data.first().map(|entry| entry.url.clone())
}

/// Looks up a real cover for a game by name via SteamGridDB, for the (very
/// common) case where the game's own Discord-IPC Rich Presence integration
/// didn't upload cover art. Only the game's display name leaves this
/// server - never who's playing it, and the API key never reaches the
/// client (SteamGridDB requests all happen server-side, this endpoint just
/// returns the resolved image URL). Self-hosters who haven't set
/// STEAMGRIDDB_API_KEY just get `{url: null}` back - no error, no crash.
pub async fn cover(
    State(state): State<AppState>,
    _session: AuthSession,
    Query(query): Query<CoverQuery>,
) -> Result<Json<CoverDto>, AppError> {
    let name = query.name.trim();
    if name.is_empty() || name.chars().count() > 128 {
        return Ok(Json(CoverDto { url: None }));
    }

    let Some(api_key) = state.game_covers.steamgriddb_api_key.clone() else {
        return Ok(Json(CoverDto { url: None }));
    };

    let cache_key = name.to_lowercase();
    if let Some(entry) = state.game_cover_cache.get(&cache_key) {
        if entry.0.elapsed() < CACHE_TTL {
            return Ok(Json(CoverDto { url: entry.1.clone() }));
        }
    }

    let url = match fetch_game_id(&state, &api_key, name).await {
        Some(game_id) => {
            let mut found = fetch_asset(&state, &api_key, "icons", game_id).await;
            if found.is_none() {
                found = fetch_asset(&state, &api_key, "grids", game_id).await;
            }
            found
        }
        None => None,
    };

    state
        .game_cover_cache
        .insert(cache_key, (Instant::now(), url.clone()));

    Ok(Json(CoverDto { url }))
}
