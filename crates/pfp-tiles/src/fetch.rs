//! reqwest client + upstream fetch helpers for OSM tile.openstreetmap.org.
//!
//! OSMF policy compliance (D-35): every upstream request carries the
//! User-Agent `photo-foot-print/<crate-version> (+contact: jykim.loa2000@gmail.com)`.
//! The `<crate-version>` is read from `env!("CARGO_PKG_VERSION")` at compile time.

use std::time::Duration;

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
