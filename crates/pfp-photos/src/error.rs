//! Typed errors for the photos pipeline. Designed to cross into src-tauri's
//! WireError without leaking internals (D-08: this crate has no tauri::*).

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PhotosError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("exif error: {0}")]
    Exif(#[from] pfp_exif::error::ExifError),

    #[error("jpeg decode error: {0}")]
    Decode(String),

    #[error("image resize error: {0}")]
    Resize(String),

    #[error("jpeg encode error: {0}")]
    Encode(String),

    #[error("thumb cache directory not writable: {0}")]
    CacheNotWritable(PathBuf),
}
