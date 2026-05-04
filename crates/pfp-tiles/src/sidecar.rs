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

/// Read the sidecar JSON. Plan 02 implements the body.
pub fn read(_path: &Path) -> Result<TileMeta, SidecarReadError> {
    unimplemented!("Plan 02 implements sidecar::read")
}

/// Atomically write the sidecar JSON via sibling tempfile + persist.
/// Plan 02 implements the body.
pub fn write_atomic(_path: &Path, _meta: &TileMeta) -> Result<(), TileError> {
    unimplemented!("Plan 02 implements sidecar::write_atomic")
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
}
