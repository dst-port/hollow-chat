use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::Response;
use dashmap::DashMap;

#[derive(Clone)]
pub struct RateLimiter {
    hits: Arc<DashMap<IpAddr, (u32, Instant)>>,
    max_attempts: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration) -> Self {
        Self {
            hits: Arc::new(DashMap::new()),
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
