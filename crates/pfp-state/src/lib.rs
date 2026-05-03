//! pfp-state -- typed AppState persisted as state.json under data_local_dir().
//!
//! D-08 invariant: zero `tauri::*` symbols. Pure Rust on top of `serde`,
//! `serde_json`, `dirs`, `tempfile`.
//! Used by:
//!   - `crates/src-tauri/src/commands/state.rs`  (Plan 04) -- `get_app_state` IPC
//!   - `crates/src-tauri/src/commands/folder.rs` (Plan 04) -- `save_last_folder` side effect on list_folder success
//!
//! Architectural rules:
//!   - Atomic save: sibling tempfile -> file fsync -> persist. NO macOS
//!     drive-cache flush step (state.json is regenerable; the gold-plated
//!     path is reserved for pfp-exif::atomic in-place EXIF writes -- see
//!     Phase 1 atomic.rs).
//!   - Forward compatibility: schema_version defaults to 1 via serde
//!     `default = "schema_version_default"`. Phase 4 will add last_pin and
//!     capture_time_default fields; reading an old (Phase 2) state.json
//!     still works because Option fields default to None.

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

mod error;
pub use error::StateError;

const APP_DIR: &str = "photo-foot-print";
const STATE_FILE: &str = "state.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppState {
    /// Schema version for forward-compat. Phase 4 will bump to 2 when adding
    /// last_pin: Option<(f64, f64)>.
    #[serde(default = "schema_version_default")]
    pub schema_version: u32,

    /// Most recently opened folder (D-12 / APP-02). None on first launch
    /// or after the user has never opened a folder.
    #[serde(default)]
    pub last_folder: Option<PathBuf>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            schema_version: schema_version_default(),
            last_folder: None,
        }
    }
}

fn schema_version_default() -> u32 {
    1
}

/// Canonical on-disk path for state.json on this OS.
/// macOS: `~/Library/Application Support/photo-foot-print/state.json`.
/// Linux: `$XDG_DATA_HOME/photo-foot-print/state.json` (or `~/.local/share/...`).
/// Windows: `%LOCALAPPDATA%\photo-foot-print\state.json`.
pub fn state_path() -> Result<PathBuf, StateError> {
    let dir = dirs::data_local_dir().ok_or(StateError::NoDataLocalDir)?;
    Ok(dir.join(APP_DIR).join(STATE_FILE))
}

/// Load AppState from the canonical state.json path.
/// Returns AppState::default() when the file does not exist.
pub fn load() -> Result<AppState, StateError> {
    load_from(&state_path()?)
}

/// Test seam: load from an explicit path. Production callers use `load()`.
pub fn load_from(path: &Path) -> Result<AppState, StateError> {
    if !path.exists() {
        return Ok(AppState::default());
    }
    let bytes = std::fs::read(path)?;
    let state: AppState =
        serde_json::from_slice(&bytes).map_err(|e| StateError::Parse(e.to_string()))?;
    Ok(state)
}

/// Save AppState atomically to the canonical state.json path.
pub fn save(state: &AppState) -> Result<(), StateError> {
    save_to(&state_path()?, state)
}

/// Test seam: save to an explicit path. Production callers use `save()`.
///
/// Atomic write contract:
///   1. Create parent dir if missing (`fs::create_dir_all`).
///   2. Open a sibling tempfile in the same dir (`.state-XXXX.json.tmp`).
///   3. Write JSON; flush; file fsync.
///   4. `persist(target)` (atomic rename on POSIX; AtomicReplaceFile on Windows).
///
/// Survives mid-write process kill: either the original is intact OR the new
/// version is fully written. No torn intermediate state.
///
/// Note: this routine deliberately omits the macOS drive-cache flush
/// step that `pfp-exif::atomic::write_via_temp` performs.
/// `state.json` is regenerable on the next `list_folder` call -- the
/// gold-plated path is reserved for in-place EXIF writes where the
/// original photo bytes cannot be recovered.
pub fn save_to(path: &Path, state: &AppState) -> Result<(), StateError> {
    let parent = path.parent().ok_or(StateError::NoParent)?;
    std::fs::create_dir_all(parent)?;

    let tmp = tempfile::Builder::new()
        .prefix(".state-")
        .suffix(".json.tmp")
        .tempfile_in(parent)?;
    {
        let mut writer = std::io::BufWriter::new(tmp.as_file());
        serde_json::to_writer_pretty(&mut writer, state)
            .map_err(|e| StateError::Parse(e.to_string()))?;
        writer.flush()?;
    }
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| StateError::Io(e.error))?;
    Ok(())
}

/// Convenience: load current state (or default), update last_folder, save.
/// This is the single entry point Plan 04's `list_folder` IPC command calls
/// after a successful folder load (D-12).
pub fn save_last_folder(folder: &Path) -> Result<(), StateError> {
    let mut state = load()?;
    state.last_folder = Some(folder.to_path_buf());
    save(&state)
}

#[cfg(test)]
mod tests {
    use crate::{AppState, StateError};

    /// AppState::default() must yield schema_version = 1 (NOT the derived 0).
    /// Plan 02-03 Task 1 RED gate.
    #[test]
    fn default_app_state_has_schema_version_1_and_no_last_folder() {
        let s = AppState::default();
        assert_eq!(s.schema_version, 1);
        assert!(s.last_folder.is_none());
    }

    /// state_path() must end in photo-foot-print/state.json under data_local_dir().
    #[test]
    fn state_path_ends_with_app_dir_and_state_json() {
        let p = crate::state_path().expect("state_path resolves on dev machine");
        let s = p.to_string_lossy();
        assert!(
            s.ends_with("photo-foot-print/state.json")
                || s.ends_with("photo-foot-print\\state.json"),
            "unexpected state_path tail: {s}",
        );
    }

    /// StateError::NoDataLocalDir is the documented failure mode for state_path()
    /// when dirs::data_local_dir() returns None. Compile-only smoke check.
    #[test]
    fn state_error_has_no_data_local_dir_variant() {
        // Construct each variant once so a future rename surfaces in the test build.
        let _ = StateError::NoDataLocalDir;
        let _ = StateError::NoParent;
        let _ = StateError::Parse("x".to_string());
    }
}
