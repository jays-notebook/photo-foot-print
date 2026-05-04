//! reqwest client + upstream fetch helpers for OSM tile.openstreetmap.org.
//!
//! OSMF policy compliance (D-35): every upstream request carries the
//! User-Agent `photo-foot-print/<crate-version> (+contact: jykim.loa2000@gmail.com)`.
//! The `<crate-version>` is read from `env!("CARGO_PKG_VERSION")` at compile time.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::header::{ETAG, IF_MODIFIED_SINCE, IF_NONE_MATCH, LAST_MODIFIED};

use crate::error::TileError;
use crate::sidecar::TileMeta;

/// Carrier for a fresh (or revalidated) tile fetched from upstream.
pub struct FreshTile {
    pub bytes: Vec<u8>,
    pub meta: TileMeta,
}

/// Build the canonical `reqwest::Client` for OSM tile fetches.
///
/// User-Agent format (D-35): `photo-foot-print/<crate-version> (+contact: jykim.loa2000@gmail.com)`
/// where `<crate-version>` is `env!("CARGO_PKG_VERSION")`.
/// Timeout: 10 seconds (per RESEARCH §Pattern 2).
pub fn build_osm_client() -> Result<reqwest::Client, TileError> {
    let ua = format!(
        "photo-foot-print/{} (+contact: jykim.loa2000@gmail.com)",
        env!("CARGO_PKG_VERSION")
    );
    reqwest::Client::builder()
        .user_agent(ua)
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| TileError::Init(e.to_string()))
}

/// Read the OSM tile at `(z, x, y)`, optionally with conditional headers
/// from a prior sidecar. Returns the `FreshTile` or a `TileError` describing
/// the failure (network, non-2xx, parse).
///
/// Cache-Control max-age is honored AND floored to 7 days per OSMF (D-32).
/// 304 Not Modified surfaces as `Err(TileError::UpstreamStatus(304))` so
/// the caller can branch (bump `fetched_at` on the existing sidecar).
pub async fn fetch_upstream(
    client: &reqwest::Client,
    z: u8,
    x: u32,
    y: u32,
    prev: Option<&TileMeta>,
) -> Result<FreshTile, TileError> {
    let url = format!("https://tile.openstreetmap.org/{z}/{x}/{y}.png");
    fetch_upstream_url(client, &url, prev).await
}

/// Same as [`fetch_upstream`] but takes the URL explicitly. Tests use this
/// to point at an httpmock server without modifying the production URL.
pub async fn fetch_upstream_url(
    client: &reqwest::Client,
    url: &str,
    prev: Option<&TileMeta>,
) -> Result<FreshTile, TileError> {
    let mut req = client.get(url);
    if let Some(p) = prev {
        if let Some(etag) = &p.etag {
            req = req.header(IF_NONE_MATCH, etag);
        }
        if let Some(lm) = &p.last_modified {
            req = req.header(IF_MODIFIED_SINCE, lm);
        }
    }
    let resp = req
        .send()
        .await
        .map_err(|e| TileError::Upstream(e.to_string()))?;
    let status = resp.status();
    if status.as_u16() == 304 {
        return Err(TileError::UpstreamStatus(304));
    }
    if !status.is_success() {
        return Err(TileError::UpstreamStatus(status.as_u16()));
    }
    // 7-day floor on Cache-Control max-age (D-32 / D-35).
    let server_max_age = parse_cache_control_max_age(resp.headers()).unwrap_or(0);
    let max_age_secs = server_max_age.max(7 * 86_400);
    let etag = resp
        .headers()
        .get(ETAG)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let last_modified = resp
        .headers()
        .get(LAST_MODIFIED)
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let bytes = resp
        .bytes()
        .await
        .map_err(|e| TileError::Upstream(e.to_string()))?
        .to_vec();
    Ok(FreshTile {
        bytes,
        meta: TileMeta {
            schema_version: 1,
            max_age_secs,
            fetched_at_unix: unix_now() as i64,
            etag,
            last_modified,
        },
    })
}

/// Parse only the `max-age=N` directive from a Cache-Control header.
/// Returns `None` if the header is absent or no `max-age=N` directive is
/// found. Per RESEARCH §"Don't Hand-Roll": targeted single-directive parse,
/// NOT a full Cache-Control grammar parser.
pub fn parse_cache_control_max_age(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let v = headers.get(reqwest::header::CACHE_CONTROL)?.to_str().ok()?;
    for directive in v.split(',') {
        let directive = directive.trim();
        if let Some(rest) = directive.strip_prefix("max-age=") {
            if let Ok(n) = rest.trim().parse::<u64>() {
                return Some(n);
            }
        }
    }
    None
}

/// Unix epoch seconds, saturating to 0 on a (deeply unlikely) clock-before-1970.
pub(crate) fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::{HeaderMap, HeaderValue, CACHE_CONTROL};

    #[test]
    fn build_osm_client_constructs_without_panic() {
        let c = build_osm_client();
        assert!(c.is_ok(), "expected Ok, got {c:?}");
    }

    fn header_with(cc: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(CACHE_CONTROL, HeaderValue::from_str(cc).unwrap());
        h
    }

    #[test]
    fn parse_max_age_happy_path() {
        let h = header_with("public, max-age=86400");
        assert_eq!(parse_cache_control_max_age(&h), Some(86_400));
    }

    #[test]
    fn parse_max_age_first_directive() {
        let h = header_with("max-age=42, public");
        assert_eq!(parse_cache_control_max_age(&h), Some(42));
    }

    #[test]
    fn parse_max_age_no_directive() {
        let h = header_with("public, no-cache");
        assert_eq!(parse_cache_control_max_age(&h), None);
    }

    #[test]
    fn parse_max_age_missing_header() {
        let h = HeaderMap::new();
        assert_eq!(parse_cache_control_max_age(&h), None);
    }

    #[test]
    fn parse_max_age_malformed_value() {
        let h = header_with("max-age=not-a-number");
        assert_eq!(parse_cache_control_max_age(&h), None);
    }
}
