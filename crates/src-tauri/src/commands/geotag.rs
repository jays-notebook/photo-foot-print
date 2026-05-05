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

/// D-40 / D-58: GPS + capture-time save. Phase 4 widens this to accept
/// `dto: Option<String>` so the frontend's `<input type="datetime-local">`
/// can route through the same single-atomic-write `pfp_exif::write_gps`
/// call. The lib already accepts `dto: Option<&str>` (Plan 04-02 extends
/// it to write `DateTimeDigitized` alongside `DateTimeOriginal`).
///
/// D-43: path canonicalization happens inside the handler. The frontend
/// sees only the path-hash `id`, never a writable path string.
///
/// Atomic-write contract (Phase 1 D-07): on any failure, the original
/// JPEG is unchanged. The error surfaces as `WireError::ExifWrite`; the
/// frontend shows it via plugin-dialog `message({kind: 'error'})`.
///
/// D-45 / D-47 last_pin write-through: on a successful
/// `pfp_exif::write_gps`, persist `(lat, lng)` to `state.json`
/// via `pfp_state::save_last_pin` BEFORE `read_detail`. State-save
/// failure is logged via `eprintln!` and swallowed — the
/// irreversible commit (the photo) already succeeded; `state.json`
/// is regenerable.
#[tauri::command]
pub async fn save_geotag(
    state: State<'_, TauriAppState>,
    id: String,
    lat: f64,
    lng: f64,
    dto: Option<String>,
) -> Result<PhotoMeta, WireError> {
    let path = resolve_id(&state, &id)?;

    // CR-01: pfp_exif::write_gps + pfp_exif::read_detail are synchronous,
    // fsync-heavy file I/O (full JPEG copy + F_FULLFSYNC + parent-dir fsync
    // + EXIF re-parse). Running them on the Tauri async runtime thread
    // would stall every sibling task — including the pfp-thumb:// and
    // pfp-tile:// URI scheme handlers that share the runtime. Mirror the
    // discipline already established in commands/thumbnail.rs:100 and
    // hand the blocking work to a dedicated thread via spawn_blocking.
    //
    // D-40 / D-58: GPS + optional capture-time. Altitude is still None
    // (no v1 UI). The dto Option<String> is cloned into the move closure
    // (cheap; up to 19 bytes) and threaded through as Option<&str>.
    // Explicit `.map_err` to ExifWrite for the write step (NOT the
    // From<ExifError> impl which would map to Exif — that lane is
    // reserved for read failures).
    let path_for_blocking = path.clone();
    let dto_for_blocking = dto.clone();
    let detail = tokio::task::spawn_blocking(move || {
        pfp_exif::write_gps(
            &path_for_blocking,
            lat,
            lng,
            None,
            dto_for_blocking.as_deref(),
        )
        .map_err(|e| WireError::ExifWrite {
            detail: e.to_string(),
        })?;

        // D-45 / D-47: persist last_pin AFTER the EXIF write succeeds,
        // BEFORE re-reading metadata. Failure is logged + swallowed —
        // the irreversible commit (the photo) already succeeded;
        // state.json is regenerable; surfacing a WireError here would
        // falsely tell the user the save failed.
        if let Err(e) = pfp_state::save_last_pin(lat, lng) {
            eprintln!("pfp_state::save_last_pin failed (best-effort): {e}");
        }

        // Re-read the file to produce fresh PhotoMeta. Reuses the Phase 2
        // read path; the frontend swaps this into its Svelte store and
        // reactivity flips the GpsBadge + ExifReadout (D-42 silent feedback).
        let detail = pfp_exif::read_detail(&path_for_blocking)?;
        Ok::<_, WireError>(detail)
    })
    .await
    .map_err(|e| WireError::Io {
        detail: format!("save_geotag join: {e}"),
    })??;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // D-27 / D-28: update session_last_pin AFTER all awaits, before return.
    // Brief acquire-mutate-release; no .await held across the guard.
    // Phase 4 (D-46): this Mutex update is now part of a write-through
    // pair — pfp_state::save_last_pin writes durably inside spawn_blocking
    // (above); this Mutex update is the in-memory hot-mirror update.
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
