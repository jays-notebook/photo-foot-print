//! Errors that cross the IPC boundary. Serializes as JSON with a `kind`
//! discriminator so the frontend can branch on `kind` rather than parsing
//! free-text strings.
//!
//! Threat model reference: T-04-04 (info disclosure). For Phase 1 dev-only
//! the `detail` strings are fine; Phase 4 polish (RESEARCH.md §"Security
//! Domain" V7) can scrub before release. The personal-use scope (D-09)
//! means no remote exposure.

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireError {
    #[error("io error: {detail}")]
    Io { detail: String },

    #[error("exif error: {detail}")]
    Exif { detail: String },

    /// Returned when an IPC `path` argument escapes the allowed root via
    /// `..` or canonicalization mismatch. Phase 1 allowed root: the bundled
    /// fixtures dir. Phase 3 will widen this to user-opened folders.
    #[error("path traversal rejected: {detail}")]
    PathTraversal { detail: String },

    /// Phase 2: pfp-photos pipeline error (folder enumeration, JPEG validation,
    /// thumbnail decode, cache write).
    #[error("photos error: {detail}")]
    Photos { detail: String },

    /// Phase 2: pfp-state persistence error (state.json read/write).
    #[error("state error: {detail}")]
    State { detail: String },

    /// Phase 3: pfp_exif::write_gps failed during save_geotag. The atomic-write
    /// contract (Phase 1 D-07) guarantees the original file is unchanged on
    /// any failure path; the frontend surfaces this via plugin-dialog message().
    #[error("exif write failed: {detail}")]
    ExifWrite { detail: String },
}

impl From<pfp_exif::error::ExifError> for WireError {
    fn from(e: pfp_exif::error::ExifError) -> Self {
        WireError::Exif {
            detail: e.to_string(),
        }
    }
}

impl From<std::io::Error> for WireError {
    fn from(e: std::io::Error) -> Self {
        WireError::Io {
            detail: e.to_string(),
        }
    }
}

impl From<pfp_photos::error::PhotosError> for WireError {
    fn from(e: pfp_photos::error::PhotosError) -> Self {
        WireError::Photos {
            detail: e.to_string(),
        }
    }
}

impl From<pfp_state::StateError> for WireError {
    fn from(e: pfp_state::StateError) -> Self {
        WireError::State {
            detail: e.to_string(),
        }
    }
}
