//! pfp-tiles -- OSM tile disk cache + reqwest fetcher with stale-while-revalidate.
//!
//! D-08 invariant: zero `tauri::*` symbols. Pure Rust on top of `reqwest`,
//! `tokio`, `serde_json`, `tempfile`. The `cache_root: PathBuf` is supplied
//! by the host (resolved via Tauri's `app.path().app_cache_dir()` in
//! `crates/src-tauri/src/lib.rs`); this crate never calls `dirs::cache_dir()`
//! directly -- keeps it testable with a tempdir.
//!
//! Used by:
//!   - `crates/src-tauri/src/protocols/tile.rs` (URI scheme handler)
//!   - `crates/src-tauri/src/lib.rs::run` (TileCache construction in setup hook)
//!
//! Architectural rules:
//!   - All on-disk paths are anchored under `cache_root`. No absolute paths in/out.
//!   - Cache layout: `{cache_root}/osm/{z}/{x}/{y}.png` + `.png.meta.json` sidecar.
//!   - Atomic writes: sibling tempfile + sync_all + persist. NO `F_FULLFSYNC`
//!     (tiles are regenerable; gold-plated path is reserved for `pfp-exif`).
//!   - User-Agent on every upstream request: OSMF policy compliance (D-35).
//!   - 7-day floor on cache freshness (D-32 / D-35).

pub mod cache;
pub mod error;
pub mod fetch;
pub mod sidecar;

pub use crate::error::TileError;
pub use crate::fetch::build_osm_client;

use std::path::PathBuf;

/// Tile cache + upstream fetcher. One per app instance (held inside
/// `TauriAppState`). Cheap to clone -- `reqwest::Client` is internally
/// `Arc`-shared and `cache_root` is just a `PathBuf`.
#[derive(Clone)]
pub struct TileCache {
    cache_root: PathBuf,
    http: reqwest::Client,
}

/// Result returned by `get_tile`.
pub struct TileResponse {
    pub bytes: Vec<u8>,
    pub from_cache: bool,
    pub was_stale: bool,
}

impl TileCache {
    /// Construct. `cache_root` MUST be the `app_cache_dir` resolved by
    /// Tauri's `app.path().app_cache_dir()` joined with `"tiles"`; this crate
    /// does not call `dirs::cache_dir()` directly so it stays testable with a
    /// tempdir.
    ///
    /// `http` MUST be built with the OSMF-compliant User-Agent -- see
    /// `build_osm_client` helper.
    pub fn new(cache_root: PathBuf, http: reqwest::Client) -> Self {
        Self { cache_root, http }
    }

    /// Fetch a tile by `(z, x, y)`. Cache-hit / cache-miss / stale-revalidate
    /// behavior is implemented in Plan 02; this signature is the contract.
    pub async fn get_tile(
        &self,
        _z: u8,
        _x: u32,
        _y: u32,
    ) -> Result<TileResponse, TileError> {
        unimplemented!("Plan 02 implements get_tile (cache hit + miss + revalidate)")
    }
}
