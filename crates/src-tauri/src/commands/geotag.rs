//! IPC: save_geotag + get_session_last_pin (Phase 3).
//!
//! Pattern 1 (Lib-First / Tauri-Thin): each #[tauri::command] is a thin
//! wrapper that delegates to pfp-exif (write + re-read) and updates one
//! Mutex field. D-08 / D-43 honored.
//!
//! Mutex discipline (CLAUDE.md / RESEARCH §Anti-Patterns / Pitfall 4):
//! never hold a Mutex across an `.await`. The `session_last_pin` mutation
//! happens AFTER `pfp_exif::write_gps` and `pfp_exif::read_detail` both
//! complete; the brief Mutex acquire is acquire-mutate-release.

use std::path::PathBuf;

use tauri::State;

use crate::app_state::TauriAppState;
use crate::commands::folder::{GpsCoord, PhotoMeta};
use crate::error::WireError;

/// Resolve `id -> PathBuf` via the managed-state id-map. Shared between
/// save_geotag and (future) any other command that takes an id arg.
fn resolve_id(state: &TauriAppState, id: &str) -> Result<PathBuf, WireError> {
    let map = state.thumbnail_id_map.lock().map_err(|e| WireError::Io {
        detail: format!("id_map lock: {e}"),
    })?;
    map.get(id).cloned().ok_or_else(|| WireError::Photos {
        detail: format!("unknown photo id: {id}"),
    })
}

/// D-40: GPS-only save. Phase 4 widens this to accept `dto: Option<String>`
/// for capture-time editing. Altitude UI is out of v1.
///
/// D-43: path canonicalization happens inside the handler. The frontend
/// sees only the path-hash `id`, never a writable path string.
///
/// Atomic-write contract (Phase 1 D-07): on any failure, the original
/// JPEG is unchanged. The error surfaces as `WireError::ExifWrite`; the
/// frontend shows it via plugin-dialog `message({kind: 'error'})`.
#[tauri::command]
pub async fn save_geotag(
    state: State<'_, TauriAppState>,
    id: String,
    lat: f64,
    lng: f64,
) -> Result<PhotoMeta, WireError> {
    let path = resolve_id(&state, &id)?;

    // D-40: GPS-only. None for altitude AND dto in Phase 3.
    // Explicit `.map_err` to ExifWrite (NOT the From<ExifError> impl
    // which would map to Exif — used for read failures).
    pfp_exif::write_gps(&path, lat, lng, None, None).map_err(|e| WireError::ExifWrite {
        detail: e.to_string(),
    })?;

    // Re-read the file to produce fresh PhotoMeta. Reuses the Phase 2
    // read path; the frontend swaps this into its Svelte store and
    // reactivity flips the GpsBadge + ExifReadout (D-42 silent feedback).
    let detail = pfp_exif::read_detail(&path)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // D-27 / D-28: update session_last_pin AFTER all awaits, before return.
    // Brief acquire-mutate-release; no .await held across the guard.
    {
        let mut last = state.session_last_pin.lock().map_err(|e| WireError::Io {
            detail: format!("session_last_pin lock: {e}"),
        })?;
        *last = Some((lat, lng));
    }

    Ok(PhotoMeta {
        id,
        file_name,
        gps: detail.gps.map(|(la, ln)| GpsCoord { lat: la, lng: ln }),
        altitude_m: detail.altitude_m,
        capture_time: detail.capture_time,
        dimensions: None,
    })
}

/// Read the current session_last_pin. Used by the frontend on photo
/// selection to decide the map's initial center for GPS-less photos
/// (D-26 / D-27 fallback chain). Returns None if no save has happened
/// yet this session.
#[tauri::command]
pub async fn get_session_last_pin(
    state: State<'_, TauriAppState>,
) -> Result<Option<GpsCoord>, WireError> {
    let last = state.session_last_pin.lock().map_err(|e| WireError::Io {
        detail: format!("session_last_pin lock: {e}"),
    })?;
    Ok(last.map(|(lat, lng)| GpsCoord { lat, lng }))
}
