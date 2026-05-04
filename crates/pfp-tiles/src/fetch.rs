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

    #[test]
    fn build_osm_client_constructs_without_panic() {
        let c = build_osm_client();
        assert!(c.is_ok(), "expected Ok, got {c:?}");
    }
}
