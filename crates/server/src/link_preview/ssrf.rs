use std::net::IpAddr;
use std::time::Duration;

use url::Url;

use crate::error::AppError;

const FETCH_TIMEOUT: Duration = Duration::from_secs(6);
const MAX_REDIRECTS: u8 = 5;
const MAX_BODY_BYTES: usize = 512 * 1024;

pub struct FetchedPage {
    pub final_url: Url,
    pub body: String,
}

fn is_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            !(v4.is_loopback()
                || v4.is_private()
                || v4.is_link_local()
                || v4.is_unspecified()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation()
                // CGNAT (100.64.0.0/10)
                || (v4.octets()[0] == 100 && (v4.octets()[1] & 0xc0) == 64)
                // benchmarking (198.18.0.0/15)
                || (v4.octets()[0] == 198 && (v4.octets()[1] & 0xfe) == 18))
        }
        IpAddr::V6(v6) => {
            if let Some(mapped) = v6.to_ipv4_mapped() {
                return is_public_ip(IpAddr::V4(mapped));
            }
            let is_unique_local = (v6.segments()[0] & 0xfe00) == 0xfc00;
            !(v6.is_loopback()
                || v6.is_unspecified()
                || v6.is_multicast()
                || is_unique_local
                // link-local fe80::/10
                || (v6.segments()[0] & 0xffc0) == 0xfe80)
        }
    }
}

async fn resolve_pinned(host: &str, port: u16) -> Result<IpAddr, AppError> {
    let addrs: Vec<_> = tokio::net::lookup_host((host, port))
        .await
        .map_err(|_| AppError::LinkNotAllowed)?
        .collect();

    if addrs.is_empty() {
        return Err(AppError::LinkNotAllowed);
    }

    for addr in &addrs {
        if !is_public_ip(addr.ip()) {
            return Err(AppError::LinkNotAllowed);
        }
    }

    Ok(addrs[0].ip())
}

fn validate_url(url: &Url) -> Result<(&str, u16), AppError> {
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(AppError::LinkNotAllowed);
    }
    let host = url.host_str().ok_or(AppError::LinkNotAllowed)?;
    if host.eq_ignore_ascii_case("localhost") {
        return Err(AppError::LinkNotAllowed);
    }
    let port = url.port_or_known_default().ok_or(AppError::LinkNotAllowed)?;
    if port != 80 && port != 443 {
        return Err(AppError::LinkNotAllowed);
    }
    Ok((host, port))
}

/// Fetches an HTML page while guarding against SSRF: only http(s) on the
/// default ports is allowed, the resolved IP must be public (no loopback,
/// private, link-local, CGNAT, etc.), the connection is pinned to that
/// resolved IP to avoid a DNS-rebind swap between check and connect, and
/// redirects are followed manually so every hop is re-validated the same
/// way instead of trusting reqwest's built-in follower.
pub async fn fetch_html(start_url: &Url) -> Result<FetchedPage, AppError> {
    let mut current = start_url.clone();

    for _ in 0..=MAX_REDIRECTS {
        let (host, port) = validate_url(&current)?;
        let pinned_ip = resolve_pinned(host, port).await?;

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(FETCH_TIMEOUT)
            .resolve(host, std::net::SocketAddr::new(pinned_ip, port))
            .build()
            .map_err(|_| AppError::LinkPreviewUnavailable)?;

        let response = client
            .get(current.clone())
            .header("User-Agent", "Mozilla/5.0 (compatible; HollowChatLinkPreview/1.0)")
            .header("Accept", "text/html")
            .send()
            .await
            .map_err(|_| AppError::LinkPreviewUnavailable)?;

        let status = response.status();
        if status.is_redirection() {
            let location = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
                .ok_or(AppError::LinkPreviewUnavailable)?;
            current = current
                .join(location)
                .map_err(|_| AppError::LinkPreviewUnavailable)?;
            continue;
        }

        if !status.is_success() {
            return Err(AppError::LinkPreviewUnavailable);
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.to_ascii_lowercase().contains("text/html") {
            return Err(AppError::LinkPreviewUnavailable);
        }

        let final_url = response.url().clone();
        let bytes = read_capped(response).await?;
        let body = String::from_utf8_lossy(&bytes).into_owned();
        return Ok(FetchedPage { final_url, body });
    }

    Err(AppError::LinkPreviewUnavailable)
}

async fn read_capped(response: reqwest::Response) -> Result<Vec<u8>, AppError> {
    use futures_util::StreamExt;

    let mut stream = response.bytes_stream();
    let mut buf = Vec::with_capacity(64 * 1024);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| AppError::LinkPreviewUnavailable)?;
        buf.extend_from_slice(&chunk);
        if buf.len() >= MAX_BODY_BYTES {
            buf.truncate(MAX_BODY_BYTES);
            break;
        }
    }

    Ok(buf)
}
