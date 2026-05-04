//! Sidecar JSON written next to each cached tile as `{z}/{x}/{y}.png.meta.json`.
//!
//! `schema_version` defaults to 1; future fields land as `Option`
//! with `#[serde(default)]` for forward-compat.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::TileError;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TileMeta {
    #[serde(default = "schema_version_default")]
    pub schema_version: u32,
    /// Effective max-age in seconds. = `max(server_cache_control_max_age, 7*86400)`.
    /// 7-day floor is the OSMF policy minimum (D-32 / D-35).
    pub max_age_secs: u64,
    /// Unix epoch seconds when the tile was last written or last revalidated
    /// (304 bumps this; 200 replaces both this and the bytes).
    pub fetched_at_unix: i64,
    /// Server-provided ETag if any (for revalidation via `If-None-Match`).
    #[serde(default)]
    pub etag: Option<String>,
    /// Server-provided Last-Modified if any (for revalidation via `If-Modified-Since`).
    #[serde(default)]
    pub last_modified: Option<String>,
}

fn schema_version_default() -> u32 {
    1
}

/// Distinguished error so callers can branch on Absent (treat as stale +
/// revalidate) vs Parse (treat as stale + log + revalidate) vs Io
/// (return cached bytes anyway). See RESEARCH §Pitfall 6.
#[derive(Debug)]
pub enum SidecarReadError {
    Absent,
    Parse(String),
    Io(std::io::Error),
}

/// Read the sidecar JSON. Distinguishes Absent / Parse / Io so callers can
/// branch (Pitfall 6: Parse is treated as stale-revalidate, NOT as miss).
pub fn read(path: &Path) -> Result<TileMeta, SidecarReadError> {
    if !path.exists() {
        return Err(SidecarReadError::Absent);
    }
    let bytes = std::fs::read(path).map_err(SidecarReadError::Io)?;
    serde_json::from_slice(&bytes).map_err(|e| SidecarReadError::Parse(e.to_string()))
}

/// Atomically write the sidecar JSON via sibling tempfile + persist.
/// The caller (`cache::write_atomic`) is responsible for ensuring the parent
/// dir exists before this is invoked, but we defensively `create_dir_all`
/// here too so standalone sidecar writes remain self-contained (covers the
/// round-trip test path that bypasses `cache::write_atomic`).
pub fn write_atomic(path: &Path, meta: &TileMeta) -> Result<(), TileError> {
    let parent = path
        .parent()
        .ok_or_else(|| TileError::Sidecar("sidecar path has no parent".to_string()))?;
    std::fs::create_dir_all(parent)?;

    let tmp = tempfile::Builder::new()
        .prefix(".tile-meta-")
        .suffix(".json.tmp")
        .tempfile_in(parent)?;
    {
        use std::io::Write;
        let mut w = std::io::BufWriter::new(tmp.as_file());
        serde_json::to_writer(&mut w, meta).map_err(|e| TileError::Sidecar(e.to_string()))?;
        w.flush()?;
    }
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| TileError::Io(e.error))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_meta_serializes_with_schema_version() {
        let m = TileMeta {
            schema_version: 1,
            max_age_secs: 604_800,
            fetched_at_unix: 0,
            etag: None,
            last_modified: None,
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"schema_version\":1"), "got: {s}");
        assert!(s.contains("\"max_age_secs\":604800"), "got: {s}");
    }

    #[test]
    fn tile_meta_deserialize_uses_defaults_for_missing_fields() {
        let m: TileMeta =
            serde_json::from_str(r#"{"max_age_secs":42,"fetched_at_unix":1}"#).unwrap();
        assert_eq!(m.schema_version, 1);
        assert_eq!(m.max_age_secs, 42);
        assert_eq!(m.fetched_at_unix, 1);
        assert!(m.etag.is_none());
        assert!(m.last_modified.is_none());
    }

    #[test]
    fn read_missing_sidecar_returns_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nope.meta.json");
        match read(&path) {
            Err(SidecarReadError::Absent) => {}
            other => panic!("expected Absent, got {other:?}"),
        }
    }

    #[test]
    fn read_corrupt_sidecar_returns_parse() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.meta.json");
        std::fs::write(&path, b"not json {{{").unwrap();
        match read(&path) {
            Err(SidecarReadError::Parse(_)) => {}
            other => panic!("expected Parse, got {other:?}"),
        }
    }

    #[test]
    fn write_atomic_then_read_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("rt.meta.json");
        let m = TileMeta {
            schema_version: 1,
            max_age_secs: 604_800,
            fetched_at_unix: 1_700_000_000,
            etag: Some("\"abc\"".to_string()),
            last_modified: Some("Wed, 21 Oct 2025 07:28:00 GMT".to_string()),
        };
        write_atomic(&path, &m).unwrap();
        let m2 = read(&path).unwrap();
        assert_eq!(m, m2);
    }
}
