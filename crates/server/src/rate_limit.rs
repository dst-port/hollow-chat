use std::hash::Hash;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::Response;
use dashmap::DashMap;
use uuid::Uuid;

/// An address or CIDR block whose `X-Forwarded-For` we believe.
///
/// Behind a reverse proxy every request arrives from the proxy, so keying a
/// limiter on the socket peer puts the whole internet in one bucket - five
/// login attempts a minute for everybody at once, and no per-attacker
/// throttling at all. `X-Forwarded-For` fixes that, but only from hops we
/// actually control: anyone may send that header, so trusting it
/// indiscriminately hands an attacker an unlimited supply of identities.
#[derive(Clone, Copy, Debug)]
pub struct TrustedProxy {
    network: IpAddr,
    prefix: u8,
}

impl TrustedProxy {
    /// Parses `10.0.0.1` or `172.16.0.0/12`. A bare address means "just this one".
    pub fn parse(spec: &str) -> Option<Self> {
        let spec = spec.trim();
        let (addr, prefix) = match spec.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix.parse::<u8>().ok()?)),
            None => (spec, None),
        };
        let network: IpAddr = addr.trim().parse().ok()?;
        let max = if network.is_ipv4() { 32 } else { 128 };
        let prefix = prefix.unwrap_or(max);
        (prefix <= max).then_some(Self { network, prefix })
    }

    fn contains(&self, ip: IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => prefix_eq(&net.octets(), &ip.octets(), self.prefix),
            (IpAddr::V6(net), IpAddr::V6(ip)) => prefix_eq(&net.octets(), &ip.octets(), self.prefix),
            _ => false,
        }
    }
}

fn prefix_eq(a: &[u8], b: &[u8], prefix: u8) -> bool {
    let whole = (prefix / 8) as usize;
    if a[..whole] != b[..whole] {
        return false;
    }
    let bits = prefix % 8;
    if bits == 0 {
        return true;
    }
    let mask = 0xffu8 << (8 - bits);
    a[whole] & mask == b[whole] & mask
}

/// The caller's real address, as far as we can honestly establish it.
///
/// Only consulted when the request actually reached us through a trusted hop.
/// We then read `X-Forwarded-For` right to left and take the first address we
/// don't control - a client may prepend whatever it likes, but the proxy
/// appends the address it really saw, so forged entries always sit further
/// left than the truth and are never reached.
pub fn client_ip(headers: &HeaderMap, peer: IpAddr, trusted: &[TrustedProxy]) -> IpAddr {
    let is_trusted = |ip: IpAddr| trusted.iter().any(|t| t.contains(ip));
    if !is_trusted(peer) {
        return peer;
    }
    let Some(raw) = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
    else {
        return peer;
    };
    for hop in raw.rsplit(',') {
        let Ok(ip) = hop.trim().parse::<IpAddr>() else {
            continue;
        };
        if !is_trusted(ip) {
            return ip;
        }
    }
    peer
}

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
    trusted_proxies: Arc<[TrustedProxy]>,
}

impl RateLimiter {
    pub fn new(max_attempts: u32, window: Duration, trusted_proxies: Arc<[TrustedProxy]>) -> Self {
        let hits = Arc::new(DashMap::new());
        spawn_pruner(hits.clone(), window);
        Self {
            hits,
            max_attempts,
            window,
            trusted_proxies,
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
    let ip = client_ip(request.headers(), addr.ip(), &limiter.trusted_proxies);
    if limiter.check(ip) {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trusted(specs: &[&str]) -> Vec<TrustedProxy> {
        specs.iter().filter_map(|s| TrustedProxy::parse(s)).collect()
    }

    fn xff(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", value.parse().unwrap());
        headers
    }

    #[test]
    fn parses_addresses_and_cidr() {
        assert!(TrustedProxy::parse("172.18.0.1").is_some());
        assert!(TrustedProxy::parse("172.16.0.0/12").is_some());
        assert!(TrustedProxy::parse("::1").is_some());
        assert!(TrustedProxy::parse("172.16.0.0/33").is_none());
        assert!(TrustedProxy::parse("not-an-ip").is_none());
    }

    #[test]
    fn cidr_covers_the_docker_bridge_range() {
        let net = TrustedProxy::parse("172.16.0.0/12").unwrap();
        assert!(net.contains("172.18.0.1".parse().unwrap()));
        assert!(net.contains("172.31.255.255".parse().unwrap()));
        assert!(!net.contains("172.32.0.1".parse().unwrap()));
        assert!(!net.contains("8.8.8.8".parse().unwrap()));
    }

    #[test]
    fn untrusted_peer_is_used_as_is() {
        // No proxy in front: the header must be ignored entirely, or anyone
        // could hand themselves a fresh bucket per request.
        let ip = client_ip(&xff("1.2.3.4"), "9.9.9.9".parse().unwrap(), &trusted(&[]));
        assert_eq!(ip, "9.9.9.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn takes_the_address_the_proxy_appended() {
        let ip = client_ip(
            &xff("9.9.9.9"),
            "172.18.0.1".parse().unwrap(),
            &trusted(&["172.16.0.0/12"]),
        );
        assert_eq!(ip, "9.9.9.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn a_forged_prefix_cannot_displace_the_real_client() {
        // The client sent "1.2.3.4"; the proxy appended what it actually saw.
        let ip = client_ip(
            &xff("1.2.3.4, 9.9.9.9"),
            "172.18.0.1".parse().unwrap(),
            &trusted(&["172.16.0.0/12"]),
        );
        assert_eq!(ip, "9.9.9.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn skips_our_own_hops_but_stops_at_the_client() {
        let ip = client_ip(
            &xff("9.9.9.9, 172.18.0.5"),
            "172.18.0.1".parse().unwrap(),
            &trusted(&["172.16.0.0/12"]),
        );
        assert_eq!(ip, "9.9.9.9".parse::<IpAddr>().unwrap());
    }

    #[test]
    fn falls_back_to_the_peer_when_the_header_is_junk() {
        let ip = client_ip(
            &xff("garbage"),
            "172.18.0.1".parse().unwrap(),
            &trusted(&["172.16.0.0/12"]),
        );
        assert_eq!(ip, "172.18.0.1".parse::<IpAddr>().unwrap());
    }

    // Async because constructing a limiter spawns its pruner task.
    #[tokio::test]
    async fn separate_clients_get_separate_buckets() {
        let limiter = RateLimiter::new(
            2,
            Duration::from_secs(60),
            trusted(&["172.16.0.0/12"]).into(),
        );
        let peer: IpAddr = "172.18.0.1".parse().unwrap();
        let a = client_ip(&xff("9.9.9.9"), peer, &limiter.trusted_proxies);
        let b = client_ip(&xff("8.8.8.8"), peer, &limiter.trusted_proxies);

        assert!(limiter.check(a));
        assert!(limiter.check(a));
        assert!(!limiter.check(a), "third attempt from the same client is refused");
        assert!(limiter.check(b), "a different client still has its own budget");
    }
}
