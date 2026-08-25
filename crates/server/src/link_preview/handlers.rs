use std::sync::LazyLock;
use std::time::{Duration, Instant};

use axum::extract::State;
use axum::Json;
use regex::Regex;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::AuthSession;
use crate::error::AppError;
use crate::state::AppState;

use super::ssrf::fetch_html;

const CACHE_TTL: Duration = Duration::from_secs(60 * 60);

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub url: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LinkPreviewDto {
    pub url: String,
    pub site_name: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

static META_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<meta\s+([^>]*?)/?>").unwrap());
static TITLE_TAG_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?is)<title[^>]*>(.*?)</title>").unwrap());
static NAME_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)(?:property|name)\s*=\s*"([^"]*)"|(?:property|name)\s*=\s*'([^']*)'"#).unwrap());
static CONTENT_ATTR_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"(?i)content\s*=\s*"([^"]*)"|content\s*=\s*'([^']*)'"#).unwrap());

static NUMERIC_ENTITY_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&#(x?)([0-9a-f]+);").unwrap());

fn decode_entities(input: &str) -> String {
    // &amp; is decoded last on purpose: unescaping it before the numeric pass
    // would turn a literal "&amp;#39;" (meant to render as the text "&#39;")
    // into a real apostrophe, double-decoding it.
    let named = input
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&nbsp;", " ");

    let numeric = NUMERIC_ENTITY_RE.replace_all(&named, |caps: &regex::Captures| {
        let is_hex = !caps[1].is_empty();
        let digits = &caps[2];
        let code = if is_hex {
            u32::from_str_radix(digits, 16).ok()
        } else {
            digits.parse::<u32>().ok()
        };
        code.and_then(char::from_u32)
            .map(|c| c.to_string())
            .unwrap_or_else(|| caps[0].to_string())
    });

    numeric.replace("&amp;", "&")
}

fn capture_one<'a>(re: &Regex, haystack: &'a str) -> Option<String> {
    let caps = re.captures(haystack)?;
    let raw = caps.get(1).or_else(|| caps.get(2))?.as_str();
    Some(decode_entities(raw.trim()))
}

fn extract_meta(html: &str, base_url: &Url) -> LinkPreviewDto {
    let mut og_title = None;
    let mut og_description = None;
    let mut og_image = None;
    let mut og_site_name = None;
    let mut twitter_title = None;
    let mut twitter_description = None;
    let mut twitter_image = None;

    for tag in META_TAG_RE.captures_iter(html) {
        let attrs = &tag[1];
        let Some(name) = capture_one(&NAME_ATTR_RE, attrs) else { continue };
        let Some(content) = capture_one(&CONTENT_ATTR_RE, attrs) else { continue };
        if content.is_empty() {
            continue;
        }

        match name.to_ascii_lowercase().as_str() {
            "og:title" => og_title = Some(content),
            "og:description" => og_description = Some(content),
            "og:image" | "og:image:url" => og_image = og_image.or(Some(content)),
            "og:site_name" => og_site_name = Some(content),
            "twitter:title" => twitter_title = Some(content),
            "twitter:description" => twitter_description = Some(content),
            "twitter:image" => twitter_image = twitter_image.or(Some(content)),
            "description" if og_description.is_none() => og_description = Some(content),
            _ => {}
        }
    }

    let title = og_title.or(twitter_title).or_else(|| {
        TITLE_TAG_RE
            .captures(html)
            .and_then(|c| c.get(1))
            .map(|m| decode_entities(m.as_str().trim()))
    });

    let image = og_image
        .or(twitter_image)
        .and_then(|raw| base_url.join(&raw).ok())
        .map(|u| u.to_string());

    LinkPreviewDto {
        url: base_url.to_string(),
        site_name: og_site_name,
        title: title.filter(|s| !s.is_empty()),
        description: og_description.or(twitter_description).filter(|s| !s.is_empty()),
        image,
    }
}

pub async fn preview(
    State(state): State<AppState>,
    _session: AuthSession,
    Json(payload): Json<PreviewRequest>,
) -> Result<Json<LinkPreviewDto>, AppError> {
    let url = Url::parse(payload.url.trim()).map_err(|_| AppError::LinkNotAllowed)?;
    let cache_key = url.to_string();

    if let Some(entry) = state.link_preview_cache.get(&cache_key) {
        if entry.0.elapsed() < CACHE_TTL {
            return Ok(Json(entry.1.clone()));
        }
    }

    let page = fetch_html(&url).await?;
    let dto = extract_meta(&page.body, &page.final_url);

    state
        .link_preview_cache
        .insert(cache_key, (Instant::now(), dto.clone()));

    Ok(Json(dto))
}
