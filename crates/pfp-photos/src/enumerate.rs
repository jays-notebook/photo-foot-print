//! Folder enumeration with strict filter pipeline (D-14, RESEARCH §Question 8).
//!
//! Filter order:
//!   1. `read_dir` entries; skip on directory iteration error -> count read_failed
//!   2. Skip dotfile entries (`.pfp-thumbs/`, `.DS_Store`, etc.) -> count non_image_hidden
//!   3. `jpeg::has_jpeg_extension` -> false: count non_image_hidden, skip
//!   4. `jpeg::is_real_jpeg` -> false: count non_image_hidden, skip (right ext, wrong magic)
//!   5. `std::fs::metadata` for size + mtime -> err: count read_failed, skip
//!   6. `pfp_exif::read_summary` -> err: count read_failed, skip
//!   7. Build PhotoEntry, push to items
//!
//! After collection: sort items by `file_name` ASC.

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::Serialize;

use crate::cache::{cache_key, ensure_cache_dir};
use crate::error::PhotosError;
use crate::jpeg::{has_jpeg_extension, is_real_jpeg};

/// Per-photo entry returned by `list_folder`.
///
/// Note: `absolute_path` is INTERNAL to the lib + IPC layer. The IPC layer
/// (Plan 04) drops it before serializing to the wire -- the JS frontend only
/// sees the `id` (path-hash indirection per Pattern 3).
#[derive(Debug, Clone, Serialize)]
pub struct PhotoEntry {
    pub id: String,                  // 32 hex chars (cache_key)
    pub file_name: String,
    pub absolute_path: PathBuf,
    pub has_gps: bool,               // strict-4 (D-21) via pfp_exif::read_summary
    pub capture_time: Option<String>,// validated DTO (D-24)
    pub size_bytes: u64,
    pub mtime_unix: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FolderFooter {
    pub total_jpegs: usize,
    pub non_image_hidden: usize,
    pub read_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderListing {
    pub folder_path: String,
    pub items: Vec<PhotoEntry>,
    pub footer: FolderFooter,
    pub thumb_cache_writable: bool,
}

/// Enumerate one (flat -- no recursion) folder of JPEGs.
///
/// `folder` MUST already be canonicalized by the caller (the IPC layer does
/// this; tests typically pass a tempdir which is already absolute).
pub fn list_folder(folder: &Path) -> Result<FolderListing, PhotosError> {
    let mut items: Vec<PhotoEntry> = Vec::new();
    let mut footer = FolderFooter::default();

    let entries = match std::fs::read_dir(folder) {
        Ok(it) => it,
        Err(e) => return Err(PhotosError::Io(e)),
    };

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => {
                footer.read_failed += 1;
                continue;
            }
        };
        let path = entry.path();
        let file_name = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => {
                footer.non_image_hidden += 1;
                continue;
            }
        };

        // Step 2: skip dotfiles (defense-in-depth -- covers .pfp-thumbs/, .DS_Store, etc.)
        if file_name.starts_with('.') {
            footer.non_image_hidden += 1;
            continue;
        }

        // Step 3: extension check.
        if !has_jpeg_extension(&path) {
            footer.non_image_hidden += 1;
            continue;
        }

        // Step 4: magic-byte sniff (catches files with .jpg ext but wrong content).
        if !is_real_jpeg(&path) {
            footer.non_image_hidden += 1;
            continue;
        }

        // Step 5: size + mtime via fs::metadata.
        let meta = match std::fs::metadata(&path) {
            Ok(m) => m,
            Err(_) => {
                footer.read_failed += 1;
                continue;
            }
        };
        if !meta.is_file() {
            footer.non_image_hidden += 1;
            continue;
        }
        let size_bytes = meta.len();
        let mtime_unix = meta
            .modified()
            .ok()
            .and_then(|st| st.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        // Step 6: read EXIF summary (strict-4 GPS + validated DTO).
        // BL-02: a JPEG with no APP1/EXIF segment at all is a valid scanner
        // output (or a `jpegtran -copy none` pass-through). Treat "EXIF absent"
        // as `has_gps: false`, `capture_time: None` and KEEP the file in the
        // listing. Only hard parse / I/O failures count as `read_failed`.
        let summary = match pfp_exif::read_summary_or_default(&path) {
            Ok(s) => s,
            Err(_) => {
                footer.read_failed += 1;
                continue;
            }
        };

        let id = cache_key(&file_name, mtime_unix, size_bytes);
        items.push(PhotoEntry {
            id,
            file_name,
            absolute_path: path,
            has_gps: summary.has_gps,
            capture_time: summary.capture_time,
            size_bytes,
            mtime_unix,
        });
    }

    items.sort_by(|a, b| a.file_name.cmp(&b.file_name));
    footer.total_jpegs = items.len();

    // D-18: probe cache writability. Loud false on PermissionDenied / ReadOnlyFilesystem;
    // any other Io error (rare) is treated as not-writable too.
    let thumb_cache_writable = ensure_cache_dir(folder).is_ok();

    Ok(FolderListing {
        folder_path: folder.to_string_lossy().into_owned(),
        items,
        footer,
        thumb_cache_writable,
    })
}
