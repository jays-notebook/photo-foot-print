//! Tauri-managed application state. Single instance shared across all IPC
//! commands and the pfp-thumb:// URI scheme handler.
//!
//! Mutex discipline (CLAUDE.md / RESEARCH §Anti-Patterns):
//!   - Acquire-mutate-release. Never hold a Mutex across an `.await`.
//!   - All four fields are independent — never grab two locks together.
//!
//! The Semaphore is initialized to `min(num_cpus, 4)` so concurrent thumbnail
//! decodes do not saturate the worker pool when the user fast-scrolls a large
//! folder (Pattern 4: Bounded Concurrency).

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use tokio::sync::Semaphore;

pub struct TauriAppState {
    /// Populated by `list_folder`; consulted by `request_thumbnail`,
    /// `read_photo_meta`, and the `pfp-thumb://` handler. JS never sees the
    /// PathBuf — it only sees the String id (path-hash indirection, Pattern 3).
    pub thumbnail_id_map: Mutex<HashMap<String, PathBuf>>,

    /// Set of ids currently being decoded; prevents duplicate work when the
    /// frontend invokes request_thumbnail twice on a fast scroll. Pitfall 2-I.
    pub in_flight: Mutex<HashSet<String>>,

    /// Bounded concurrency for thumbnail decoding (Pattern 4).
    pub thumb_semaphore: Arc<Semaphore>,

    /// In-memory mirror of state.json. IPC commands mutate this; persistence
    /// happens via pfp_state::save (atomic) on every mutation.
    pub app_state: Mutex<pfp_state::AppState>,
}

impl Default for TauriAppState {
    fn default() -> Self {
        // Permits = min(num_cpus, 4). std::thread::available_parallelism is
        // stable since 1.59; falls back to 4 on error (the cap anyway).
        let permits = std::thread::available_parallelism()
            .map(|n| n.get().min(4))
            .unwrap_or(4);

        // On-startup load. A corrupt state.json must NOT block startup —
        // we silently fall back to default and let the user re-open a folder.
        let app_state = pfp_state::load().unwrap_or_default();

        Self {
            thumbnail_id_map: Mutex::new(HashMap::new()),
            in_flight: Mutex::new(HashSet::new()),
            thumb_semaphore: Arc::new(Semaphore::new(permits)),
            app_state: Mutex::new(app_state),
        }
    }
}
