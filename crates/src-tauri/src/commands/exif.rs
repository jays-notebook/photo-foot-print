//! IPC commands for EXIF read. Pattern 1 from RESEARCH.md / ARCHITECTURE.md:
//! each command is a thin wrapper that delegates into the `pfp-exif` lib
//! crate, plus path validation at the boundary.
//!
//! D-08 invariant: this file is the ONLY place where `tauri::*` and
//! `pfp_exif::*` meet; the lib crate carries zero `tauri::*` symbols.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::WireError;

/// Phase 1 IPC payload for `read_exif_summary`. Mirrors RESEARCH.md
/// §"Code Examples §5".
#[derive(Debug, Clone, Serialize)]
pub struct SummaryDto {
    pub path: String,
    pub file_name: String,
    pub has_gps: bool,
    pub capture_time: Option<String>,
}

/// Resolve the bundled fixture directory.
///
/// In `cargo tauri dev` (D-04): `CARGO_MANIFEST_DIR/../../tests/fixtures`.
/// `CARGO_MANIFEST_DIR == crates/src-tauri`, so two `..` segments reach the
/// repo root.
///
/// In a built bundle (Phase 2 work): use `tauri::api::path::resolve_resource`.
/// Phase 1 is dev-only by design (D-09 / RESEARCH.md §"Recommendation: Fixture
/// Sourcing"), so we hard-code the dev path. A future phase that ships a built
/// bundle will replace this fn body.
fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

/// Validate that `path_str` resolves inside the bundled fixtures dir.
///
/// Mitigates security threat T-04-01 (path traversal via the IPC argument).
/// The IPC command accepts an arbitrary `String`, but the only Phase 1 callers
/// (the frontend's `read_exif_summary` invocations) feed paths returned by
/// `list_fixtures`. This validator enforces the contract regardless of caller.
///
/// Phase 3 will widen the allowed root to user-opened folders; for now, allow
/// only paths that canonicalize INTO the fixtures dir.
fn assert_inside_fixtures_root(path_str: &str) -> Result<PathBuf, WireError> {
    let p = Path::new(path_str);
    let canonical = p
        .canonicalize()
        .map_err(|e| WireError::PathTraversal {
            detail: format!("canonicalize {path_str}: {e}"),
        })?;
    let root_canonical = fixtures_root()
        .canonicalize()
        .map_err(|e| WireError::PathTraversal {
            detail: format!("canonicalize fixtures_root: {e}"),
        })?;
    if !canonical.starts_with(&root_canonical) {
        return Err(WireError::PathTraversal {
            detail: format!("{path_str} is not inside the fixtures dir"),
        });
    }
    Ok(canonical)
}

#[tauri::command]
pub fn read_exif_summary(path: String) -> Result<SummaryDto, WireError> {
    let canonical = assert_inside_fixtures_root(&path)?;
    let summary = pfp_exif::read_summary(&canonical)?;
    Ok(SummaryDto {
        path: canonical.to_string_lossy().into_owned(),
        file_name: canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string(),
        has_gps: summary.has_gps,
        capture_time: summary.capture_time,
    })
}

#[tauri::command]
pub fn list_fixtures() -> Result<Vec<String>, WireError> {
    let mut paths = Vec::new();
    let root = fixtures_root();
    if !root.is_dir() {
        // No fixtures dir at all -- return empty list rather than error so the
        // frontend renders an empty-state hint.
        return Ok(paths);
    }
    for vendor in ["sony", "canon", "nikon"] {
        let dir = root.join(vendor);
        if !dir.is_dir() {
            continue;
        }
        let entries = std::fs::read_dir(&dir)?;
        for entry in entries {
            let entry = entry?;
            let p = entry.path();
            // Skip Git LFS pointer files (typically ~130 bytes) so the demo
            // does not claim they are real JPEGs.
            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            if size <= 1024 {
                continue;
            }
            let is_jpg = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"))
                .unwrap_or(false);
            if is_jpg {
                paths.push(p.to_string_lossy().into_owned());
            }
        }
    }
    paths.sort();
    Ok(paths)
}
