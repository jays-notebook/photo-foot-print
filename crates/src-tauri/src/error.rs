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
