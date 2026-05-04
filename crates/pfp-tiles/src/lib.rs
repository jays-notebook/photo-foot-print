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
pub use crate::fetch::{build_osm_client, fetch_upstream_url, FreshTile};

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

    /// Fetch a tile by `(z, x, y)`.
    ///
    /// Behavior:
    /// - Cache hit (tile + fresh sidecar): return cached bytes, no upstream
    ///   request. `from_cache: true, was_stale: false`.
    /// - Stale (sidecar age exceeds max_age_secs): return cached bytes
    ///   immediately AND spawn a background revalidation task. The caller
    ///   is unblocked the moment we have bytes; the next `get_tile` call
    ///   typically observes a fresh sidecar.
    ///   `from_cache: true, was_stale: true`.
    /// - Sidecar Absent or Parse error (Pitfall 6): treated as STALE, NOT
    ///   as miss. Cache self-heals via the spawned revalidation.
    /// - Sidecar Io error (transient FS hiccup): return cached bytes,
    ///   skip revalidation. Treated as fresh-enough for this call.
    /// - Cache miss (no tile file): synchronously fetch upstream, write
    ///   tile + sidecar atomically, return bytes.
    ///   `from_cache: false, was_stale: false`.
    pub async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<TileResponse, TileError> {
        let tile_path = crate::cache::tile_path(&self.cache_root, z, x, y);
        let meta_path = crate::cache::meta_path(&self.cache_root, z, x, y);

        // Cache hit branch (tile file exists). Sidecar may be Absent / Parse / Io.
        if tile_path.exists() {
            let meta_result = crate::sidecar::read(&meta_path);
            let now = crate::fetch::unix_now();
            let (meta_for_revalidate, stale) = match meta_result {
                Ok(m) => {
                    let stale = now.saturating_sub(m.fetched_at_unix as u64) > m.max_age_secs;
                    (Some(m), stale)
                }
                // Pitfall 6: Absent and Parse both treated as stale -> revalidate.
                Err(crate::sidecar::SidecarReadError::Absent) => (None, true),
                Err(crate::sidecar::SidecarReadError::Parse(_)) => (None, true),
                // Transient FS hiccup -> return cached bytes, do NOT refetch.
                Err(crate::sidecar::SidecarReadError::Io(_)) => (None, false),
            };

            let bytes = std::fs::read(&tile_path)?;
            if stale {
                // Spawn revalidation. Do NOT await -- caller already has bytes.
                let this = self.clone();
                let prev = meta_for_revalidate.clone();
                tokio::spawn(async move {
                    let _ = this.revalidate(z, x, y, prev).await;
                });
            }
            return Ok(TileResponse {
                bytes,
                from_cache: true,
                was_stale: stale,
            });
        }

        // Cache miss branch -- synchronous upstream fetch.
        let fresh = crate::fetch::fetch_upstream(&self.http, z, x, y, None).await?;
        crate::cache::write_atomic(&tile_path, &meta_path, &fresh.bytes, &fresh.meta)?;
        Ok(TileResponse {
            bytes: fresh.bytes,
            from_cache: false,
            was_stale: false,
        })
    }

    /// Background revalidation. Errors are eaten -- the caller already has
    /// stale bytes; on transient failure we keep the cache as-is and try
    /// again on the next stale read.
    async fn revalidate(
        &self,
        z: u8,
        x: u32,
        y: u32,
        prev: Option<crate::sidecar::TileMeta>,
    ) -> Result<(), TileError> {
        let tile_path = crate::cache::tile_path(&self.cache_root, z, x, y);
        let meta_path = crate::cache::meta_path(&self.cache_root, z, x, y);
        match crate::fetch::fetch_upstream(&self.http, z, x, y, prev.as_ref()).await {
            Ok(fresh) => {
                crate::cache::write_atomic(&tile_path, &meta_path, &fresh.bytes, &fresh.meta)?;
            }
            Err(TileError::UpstreamStatus(304)) => {
                // 304 Not Modified -- bump fetched_at on the existing sidecar.
                if let Some(mut p) = prev {
                    p.fetched_at_unix = crate::fetch::unix_now() as i64;
                    crate::sidecar::write_atomic(&meta_path, &p)?;
                }
            }
            Err(_) => {
                // Network down / 5xx / parse error -- cache stays as-is. v1
                // is eprintln-only logging; structured logging is post-v1.
                eprintln!("pfp-tiles: revalidate {z}/{x}/{y} failed (cache kept)");
            }
        }
        Ok(())
    }
}
