// Local HTTP bridge for the HollowChat Media Presence browser extension.
//
// The extension's content scripts read the Media Session API (or a light
// DOM fallback) on youtube.com / soundcloud.com / open.spotify.com /
// twitch.tv and POST what's currently playing to this server. It's the
// same trust model as `rpc.rs`'s Discord-IPC bridge: any local process can
// reach it, but it only listens on 127.0.0.1 and just feeds the same
// presence pipeline games already use.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Deserialize;

use crate::rpc::PresenceUpdate;

pub const PORT: u16 = 47821;
const IDLE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_FIELD_LEN: usize = 256;

#[derive(Debug, Deserialize)]
struct MediaPresenceBody {
    source: String,
    title: String,
    #[serde(default)]
    subtitle: Option<String>,
    #[serde(default = "default_true")]
    playing: bool,
}

fn default_true() -> bool {
    true
}

fn truncate(text: &str) -> String {
    text.chars().take(MAX_FIELD_LEN).collect()
}

fn source_label(source: &str) -> &'static str {
    match source {
        "youtube" => "YouTube",
        "soundcloud" => "SoundCloud",
        "spotify" => "Spotify",
        "twitch" => "Twitch",
        _ => "Browser",
    }
}

fn source_verb(source: &str) -> &'static str {
    match source {
        "youtube" | "twitch" => "Watching",
        _ => "Listening to",
    }
}

fn build_update(body: &MediaPresenceBody) -> Option<PresenceUpdate> {
    if !body.playing || body.title.trim().is_empty() {
        return None;
    }
    Some(PresenceUpdate {
        application_id: None,
        details: Some(truncate(&format!(
            "{} {}",
            source_verb(&body.source),
            body.title.trim()
        ))),
        state: body
            .subtitle
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(truncate),
        large_text: Some(source_label(&body.source).to_string()),
        large_image: None,
        small_image: None,
        small_text: None,
        start_timestamp: None,
        party_size: None,
        party_max: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_update_for_playing_media() {
        let body = MediaPresenceBody {
            source: "youtube".to_string(),
            title: "Some Video".to_string(),
            subtitle: Some("Some Channel".to_string()),
            playing: true,
        };
        let update = build_update(&body).expect("should build an update");
        assert_eq!(update.details.as_deref(), Some("Watching Some Video"));
        assert_eq!(update.state.as_deref(), Some("Some Channel"));
        assert_eq!(update.large_text.as_deref(), Some("YouTube"));
    }

    #[test]
    fn clears_when_not_playing() {
        let body = MediaPresenceBody {
            source: "soundcloud".to_string(),
            title: "A Track".to_string(),
            subtitle: None,
            playing: false,
        };
        assert!(build_update(&body).is_none());
    }

    #[test]
    fn clears_when_title_blank() {
        let body = MediaPresenceBody {
            source: "spotify".to_string(),
            title: "   ".to_string(),
            subtitle: None,
            playing: true,
        };
        assert!(build_update(&body).is_none());
    }

    #[test]
    fn unknown_source_falls_back_to_browser_label() {
        let body = MediaPresenceBody {
            source: "some-other-site".to_string(),
            title: "Whatever".to_string(),
            subtitle: None,
            playing: true,
        };
        let update = build_update(&body).unwrap();
        assert_eq!(update.large_text.as_deref(), Some("Browser"));
    }

    #[test]
    fn only_extension_and_local_origins_are_allowed() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("chrome-extension://abcdefghijklmnop")));
        assert!(origin_allowed(Some("moz-extension://abcdefghijklmnop")));

        assert!(!origin_allowed(Some("https://evil.example")));
        assert!(!origin_allowed(Some("http://localhost:3000")));
        assert!(!origin_allowed(Some("null")));
    }
}

/// Which origins may talk to the bridge.
///
/// A web page the user happens to visit can reach a loopback server just as
/// easily as the extension can, and a wildcard `Access-Control-Allow-Origin`
/// used to let it through - so any page with a stray ad script could set the
/// user's broadcast presence to text of its choosing. Browsers forbid pages
/// from forging `Origin`, so refusing http(s) origins closes that off while
/// leaving the extension (`chrome-extension://` / `moz-extension://`, whose id
/// varies per install and so can't be pinned here) working.
///
/// A request with no `Origin` at all is a non-browser local client, which is
/// the trust model this bridge always had. `null` is explicitly not allowed:
/// that's what a sandboxed iframe or a `data:` document sends.
fn origin_allowed(origin: Option<&str>) -> bool {
    match origin {
        None => true,
        Some(origin) => {
            let origin = origin.trim();
            origin.starts_with("chrome-extension://")
                || origin.starts_with("moz-extension://")
                || origin.starts_with("safari-web-extension://")
        }
    }
}

fn origin_of(request: &tiny_http::Request) -> Option<String> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Origin"))
        .map(|h| h.value.as_str().to_string())
}

/// Echo back only the caller's own (already allowlisted) origin - never `*`.
fn cors_headers(origin: Option<&str>) -> Vec<tiny_http::Header> {
    let mut headers = vec![
        tiny_http::Header::from_bytes(&b"Vary"[..], &b"Origin"[..]).unwrap(),
    ];
    if let Some(origin) = origin {
        if let Ok(header) =
            tiny_http::Header::from_bytes(&b"Access-Control-Allow-Origin"[..], origin.as_bytes())
        {
            headers.push(header);
        }
    }
    headers
}

fn respond_empty(request: tiny_http::Request, status: u16, origin: Option<&str>) {
    let mut response = tiny_http::Response::empty(status);
    for header in cors_headers(origin) {
        response.add_header(header);
    }
    let _ = request.respond(response);
}

/// Starts the local media-presence HTTP server in the background.
/// `on_update` mirrors `rpc::spawn`'s callback: `Some(..)` when the
/// extension reports something playing, `None` when it stops or goes
/// quiet for longer than `IDLE_TIMEOUT`.
pub fn spawn<F>(on_update: F)
where
    F: Fn(Option<PresenceUpdate>) + Send + Sync + Clone + 'static,
{
    std::thread::spawn(move || {
        let server = match tiny_http::Server::http(("127.0.0.1", PORT)) {
            Ok(server) => server,
            Err(_) => return,
        };

        let last_seen = Arc::new(Mutex::new(Instant::now()));
        {
            let last_seen = last_seen.clone();
            let watchdog_cb = on_update.clone();
            let mut cleared = false;
            std::thread::spawn(move || loop {
                std::thread::sleep(Duration::from_secs(5));
                let idle = last_seen
                    .lock()
                    .map(|t| t.elapsed() > IDLE_TIMEOUT)
                    .unwrap_or(false);
                if idle && !cleared {
                    watchdog_cb(None);
                    cleared = true;
                } else if !idle {
                    cleared = false;
                }
            });
        }

        for mut request in server.incoming_requests() {
            let origin = origin_of(&request);

            // Refused before anything else is looked at, so a disallowed
            // caller can't even use the response code to probe for the app.
            if !origin_allowed(origin.as_deref()) {
                respond_empty(request, 403, None);
                continue;
            }

            if matches!(request.method(), tiny_http::Method::Options) {
                let mut response = tiny_http::Response::empty(204)
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Methods"[..],
                            &b"POST, OPTIONS"[..],
                        )
                        .unwrap(),
                    )
                    .with_header(
                        tiny_http::Header::from_bytes(
                            &b"Access-Control-Allow-Headers"[..],
                            &b"Content-Type"[..],
                        )
                        .unwrap(),
                    );
                for header in cors_headers(origin.as_deref()) {
                    response.add_header(header);
                }
                let _ = request.respond(response);
                continue;
            }

            if !matches!(request.method(), tiny_http::Method::Post) || request.url() != "/presence"
            {
                respond_empty(request, 404, origin.as_deref());
                continue;
            }

            let mut body = String::new();
            if request.as_reader().read_to_string(&mut body).is_err() {
                respond_empty(request, 400, origin.as_deref());
                continue;
            }

            match serde_json::from_str::<MediaPresenceBody>(&body) {
                Ok(parsed) => {
                    if let Ok(mut seen) = last_seen.lock() {
                        *seen = Instant::now();
                    }
                    on_update(build_update(&parsed));
                    respond_empty(request, 204, origin.as_deref());
                }
                Err(_) => respond_empty(request, 400, origin.as_deref()),
            }
        }
    });
}
