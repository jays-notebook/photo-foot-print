//! Typed errors for the EXIF read+write path. Designed to cross into src-tauri's
//! WireError without leaking internals (D-08: this crate has no tauri::*).

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExifError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// little_exif's error type is `std::io::Error`, but we string-format here so
    /// any future change in upstream's error surface does not ripple into our API.
    #[error("little_exif error: {0}")]
    LittleExif(String),

    #[error("invalid DateTimeOriginal format: {0} (expected YYYY:MM:DD HH:MM:SS)")]
    InvalidDateTime(String),

    #[error("path has no parent directory: {0}")]
    NoParent(PathBuf),
}
