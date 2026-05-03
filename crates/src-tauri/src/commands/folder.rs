//! IPC commands for folder open + listing + per-photo metadata. Pattern 1
//! (Lib-First / Tauri-Thin): each command is a thin wrapper that delegates
//! into pfp-photos / pfp-exif / pfp-state.
//!
//! D-08 invariant: this file is one of the ONLY places where `tauri::*` and
//! `pfp_photos::* / pfp_exif::* / pfp_state::*` meet; the lib crates carry
//! zero `tauri::*` symbols.

use std::path::PathBuf;

use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::app_state::TauriAppState;
use crate::error::WireError;

// ============================================================================
// Wire DTOs
// ============================================================================

#[derive(Debug, Clone, Serialize)]
pub struct PhotoSummary {
    pub id: String,                   // 32 hex chars (sha256 prefix)
    pub file_name: String,
    pub has_gps: bool,                // strict-4 (D-21)
    pub capture_time: Option<String>, // validated DTO (D-24)
    pub size_bytes: u64,
    pub mtime_unix: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderFooter {
    pub total_jpegs: usize,
    pub non_image_hidden: usize,
    pub read_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderListing {
    pub folder_path: String,        // canonicalized, for the toolbar label
    pub items: Vec<PhotoSummary>,
    pub footer: FolderFooter,
    pub thumb_cache_writable: bool, // false → frontend shows D-18 banner
}

#[derive(Debug, Clone, Serialize)]
pub struct GpsCoord {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PhotoMeta {
    pub id: String,
    pub file_name: String,
    pub gps: Option<GpsCoord>,
    pub altitude_m: Option<f64>,
    pub capture_time: Option<String>,
    pub dimensions: Option<(u32, u32)>, // None in Phase 2 (UI-SPEC does not surface)
}

// ============================================================================
// Commands
// ============================================================================

/// Open the system folder picker. Returns Some(canonicalized path string) on
/// selection, None on cancel. The dialog is proxied through Rust so JS does
/// not need to import `@tauri-apps/plugin-dialog` directly (preserves D-04
/// "Rust owns I/O" boundary).
#[tauri::command]
pub async fn open_folder_dialog(app: tauri::AppHandle) -> Result<Option<String>, WireError> {
    // tauri-plugin-dialog 2.x exposes a callback API. We bridge to async via a
    // oneshot channel.
    let (tx, rx) = tokio::sync::oneshot::channel::<Option<PathBuf>>();
    app.dialog().file().pick_folder(move |opt| {
        // Convert FilePath to PathBuf. FilePath::Path holds a std::path::PathBuf;
        // FilePath::Url is mobile-only on this codebase (we ignore for desktop).
        let pb = opt.and_then(|fp| match fp {
            tauri_plugin_dialog::FilePath::Path(p) => Some(p),
            // Mobile URI variant — desktop never returns this.
            _ => None,
        });
        let _ = tx.send(pb);
    });

    let chosen = rx.await.map_err(|e| WireError::Io {
        detail: format!("dialog channel: {e}"),
    })?;

    let Some(folder) = chosen else {
        return Ok(None);
    };
    let canonical = folder.canonicalize().map_err(|e| WireError::Io {
        detail: format!("canonicalize folder: {e}"),
    })?;
    Ok(Some(canonical.to_string_lossy().into_owned()))
}

/// Enumerate JPEGs in a folder. Side effects:
///   1. Populates `state.thumbnail_id_map` with the new (id -> PathBuf) entries.
///   2. Calls `pfp_state::save_last_folder` to persist for D-13 hybrid startup.
#[tauri::command]
pub async fn list_folder(
    state: State<'_, TauriAppState>,
    path: String,
) -> Result<FolderListing, WireError> {
    let canonical: PathBuf = PathBuf::from(&path)
        .canonicalize()
        .map_err(|e| WireError::Io {
            detail: format!("canonicalize folder: {e}"),
        })?;

    // The lib crate does the work (filter pipeline, sort, cache writability probe).
    let listing = pfp_photos::list_folder(&canonical)?;

    // Populate id-map so request_thumbnail / read_photo_meta / pfp-thumb:// resolve.
    {
        let mut map = state.thumbnail_id_map.lock().map_err(|e| WireError::Io {
            detail: format!("id_map lock: {e}"),
        })?;
        map.clear();
        for entry in &listing.items {
            map.insert(entry.id.clone(), entry.absolute_path.clone());
        }
    }

    // Persist last-opened folder. Do not propagate an error here — best-effort.
    if let Err(e) = pfp_state::save_last_folder(&canonical) {
        eprintln!("[list_folder] save_last_folder failed: {e}");
    }

    // Mirror to in-memory app_state so get_app_state stays consistent without
    // re-reading disk.
    {
        let mut s = state.app_state.lock().map_err(|e| WireError::Io {
            detail: format!("app_state lock: {e}"),
        })?;
        s.last_folder = Some(canonical.clone());
    }

    // Convert internal lib types to wire types (drops absolute_path).
    Ok(FolderListing {
        folder_path: canonical.to_string_lossy().into_owned(),
        items: listing
            .items
            .into_iter()
            .map(|e| PhotoSummary {
                id: e.id,
                file_name: e.file_name,
                has_gps: e.has_gps,
                capture_time: e.capture_time,
                size_bytes: e.size_bytes,
                mtime_unix: e.mtime_unix,
            })
            .collect(),
        footer: FolderFooter {
            total_jpegs: listing.footer.total_jpegs,
            non_image_hidden: listing.footer.non_image_hidden,
            read_failed: listing.footer.read_failed,
        },
        thumb_cache_writable: listing.thumb_cache_writable,
    })
}

/// Read full EXIF metadata for the detail pane. Resolves opaque id via the
/// id-map populated by `list_folder`.
#[tauri::command]
pub async fn read_photo_meta(
    state: State<'_, TauriAppState>,
    id: String,
) -> Result<PhotoMeta, WireError> {
    let path = {
        let map = state.thumbnail_id_map.lock().map_err(|e| WireError::Io {
            detail: format!("id_map lock: {e}"),
        })?;
        map.get(&id).cloned()
    };

    let path = path.ok_or_else(|| WireError::Photos {
        detail: format!("unknown photo id: {id}"),
    })?;

    let detail = pfp_exif::read_detail(&path)?;

    let file_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    Ok(PhotoMeta {
        id,
        file_name,
        gps: detail.gps.map(|(lat, lng)| GpsCoord { lat, lng }),
        altitude_m: detail.altitude_m,
        capture_time: detail.capture_time,
        dimensions: None, // UI-SPEC does not require this in Phase 2.
    })
}
