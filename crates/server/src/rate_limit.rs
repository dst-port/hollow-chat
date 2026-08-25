use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use dashmap::DashMap;
use uuid::Uuid;

fn spawn_pruner<K>(hits: Arc<DashMap<K, (u32, Instant)>>, window: Duration)
where
    K: Eq + Hash + Send + Sync + 'static,
{
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(window.max(Duration::from_secs(1)) * 2);
        loop {
            interval.tick().await;
            hits.retain(|_, (_, last)| last.elapsed() < window);
        }
    });
}

#[derive(Clone)]
pub struct RateLimiter {
    hits: Arc<DashMap<IpAddr, (u32, Instant)>>,
    max_attempts: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        let hits = Arc::new(DashMap::new());
        spawn_pruner(hits.clone(), window);
        Self {
            hits,
            max_attempts,
            window,
        }
    }

    fn check(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let mut entry = self.hits.entry(ip).or_insert((0, now));

        if now.duration_since(entry.1) > self.window {
            *entry = (0, now);
        }

        entry.0 += 1;
        entry.0 <= self.max_attempts
    }
}

#[derive(Clone)]
pub struct UserRateLimiter {
    hits: Arc<DashMap<Uuid, (u32, Instant)>>,
    max_attempts: u32,
    window: Duration,
}

impl UserRateLimiter {
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        let hits = Arc::new(DashMap::new());
        spawn_pruner(hits.clone(), window);
        Self {
            hits,
            max_attempts,
            window,
        }
    }

    pub fn check(&self, user_id: Uuid) -> bool {
        let now = Instant::now();
        let mut entry = self.hits.entry(user_id).or_insert((0, now));

        if now.duration_since(entry.1) > self.window {
            *entry = (0, now);
        }

        entry.0 += 1;
        entry.0 <= self.max_attempts
    }
}

pub async fn enforce(
    State(limiter): State<RateLimiter>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if limiter.check(addr.ip()) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}
