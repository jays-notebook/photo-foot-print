//! Tauri-managed application state. Single instance shared across all IPC
//! commands and the pfp-thumb:// + pfp-tile:// URI scheme handlers.
//!
//! Mutex discipline (CLAUDE.md / RESEARCH §Anti-Patterns):
//!   - Acquire-mutate-release. Never hold a Mutex across an `.await`.
//!   - All independent fields — never grab two locks together.
//!
//! Phase 2 thumb_semaphore: `min(num_cpus, 4)` so concurrent thumbnail
//! decodes do not saturate the worker pool. Phase 3 tile_fetch_semaphore:
//! fixed 4 permits — caps concurrent OSM upstream fetches per OSMF policy
//! (D-35).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

pub struct TauriAppState {
    /// Populated by `list_folder`; consulted by `request_thumbnail`,
    /// `read_photo_meta`, `save_geotag`, and the `pfp-thumb://` handler.
    /// JS never sees the PathBuf — it only sees the String id.
    pub thumbnail_id_map: Mutex<HashMap<String, PathBuf>>,

    /// Set of ids currently being decoded; prevents duplicate work.
    pub in_flight: Mutex<HashSet<String>>,

    /// Bounded concurrency for thumbnail decoding (Phase 2).
    pub thumb_semaphore: Arc<Semaphore>,

    /// In-memory mirror of state.json. IPC commands mutate this; persistence
    /// happens via pfp_state::save (atomic) on every mutation.
    pub app_state: Mutex<pfp_state::AppState>,

    /// Phase 3 (D-28): last successfully-saved coordinates within this app
    /// session. Re-set on every successful save_geotag. None on app start;
    /// destroyed on app exit. Phase 4 will move this to pfp-state for
    /// cross-launch persistence.
    pub session_last_pin: Mutex<Option<(f64, f64)>>,

    /// Phase 3 (D-30): shared tile cache + reqwest fetcher. Constructed once
    /// in the Builder `.setup` hook against `app.path().app_cache_dir()`.
    pub tiles: pfp_tiles::TileCache,

    /// Phase 3 (D-35 OSMF policy): cap on concurrent upstream tile fetches.
    /// Permits = 4 (matches recommend in RESEARCH §Pattern 3). Held only
    /// across `state.tiles.get_tile(z, x, y).await` in protocols/tile.rs;
    /// cache hits return fast and free the permit immediately.
    pub tile_fetch_semaphore: Arc<Semaphore>,
}

impl TauriAppState {
    /// Production constructor — used by `lib.rs::run`'s `.setup` hook.
    /// Takes the externally-resolved `TileCache` (constructed against
    /// the real `app_cache_dir()`).
    pub fn with_tiles(tiles: pfp_tiles::TileCache) -> Self {
        let permits = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(4);
        let app_state = pfp_state::load().unwrap_or_default();
        Self {
            thumbnail_id_map: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
            thumb_semaphore: Arc::new(Semaphore::new(permits)),
            app_state: Mutex::new(app_state),
            session_last_pin: Mutex::new(None),
            tiles,
            tile_fetch_semaphore: Arc::new(Semaphore::new(4)),
        }
    }
}

impl Default for TauriAppState {
    fn default() -> Self {
        // Test-only: wire a TileCache against a tempdir + the OSMF reqwest
        // client. The production path goes through `with_tiles`; tests
        // exercise this default freely. `build_osm_client` only fails on
        // unrecoverable TLS-stack init errors which would also break any
        // production startup, so `.expect()` is acceptable here.
        //
        // D-08 cleanliness: src-tauri does NOT take a direct `reqwest` dep;
        // we go through pfp-tiles' public API surface for client construction.
        let tiles = pfp_tiles::TileCache::new(
            std::env::temp_dir().join("pfp-tiles-test-cache"),
            pfp_tiles::build_osm_client().expect("build_osm_client (test default)"),
        );
        Self::with_tiles(tiles)
    }
}
