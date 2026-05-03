//! Folder-local thumbnail cache (D-17, D-18).
//!
//! Cache layout: `<photo-folder>/.pfp-thumbs/<sha256(file_name||mtime||size)[:16]>.jpg`
//!
//! Atomic write: sibling tempfile -> sync_all -> persist -> parent fsync.
//! No `F_FULLFSYNC`: thumbnails are regenerable, the in-place gold-plated
//! durability path is reserved for `pfp-exif::atomic::write_via_temp` (Phase 1).

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::PhotosError;

pub const THUMB_CACHE_DIR: &str = ".pfp-thumbs";

/// Cache key = sha256(file_name || "|" || mtime_unix_le || "|" || size_le)[:16] hex.
/// 16 bytes = 32 hex chars = 128 bits. Collision-safe at v1 scale (flat folder
/// with unique filenames). See Pitfall 2-H.
pub fn cache_key(file_name: &str, mtime_unix: i64, size_bytes: u64) -> String {
    let mut h = Sha256::new();
    h.update(file_name.as_bytes());
    h.update(b"|");
    h.update(mtime_unix.to_le_bytes());
    h.update(b"|");
    h.update(size_bytes.to_le_bytes());
    let digest = h.finalize();
    hex_lowercase(&digest[..16])
}

pub fn cache_dir(folder: &Path) -> PathBuf {
    folder.join(THUMB_CACHE_DIR)
}

pub fn cache_file(folder: &Path, key: &str) -> PathBuf {
    cache_dir(folder).join(format!("{key}.jpg"))
}

/// Probe whether the cache directory can be created. Returns Ok(()) on success
/// (or if it already exists), `Err(PhotosError::CacheNotWritable)` on permission
/// error or read-only filesystem (D-18 banner trigger).
pub fn ensure_cache_dir(folder: &Path) -> Result<(), PhotosError> {
    let dir = cache_dir(folder);
    match std::fs::create_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e)
            if matches!(
                e.kind(),
                std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::ReadOnlyFilesystem
            ) =>
        {
            Err(PhotosError::CacheNotWritable(folder.to_path_buf()))
        }
        Err(e) => Err(PhotosError::Io(e)),
    }
}

/// Atomic write: sibling tempfile in the cache dir, sync_all, persist, parent fsync.
pub fn write_atomic(folder: &Path, key: &str, bytes: &[u8]) -> Result<(), PhotosError> {
    ensure_cache_dir(folder)?;
    let dir = cache_dir(folder);
    let target = cache_file(folder, key);

    let mut tmp = tempfile::Builder::new()
        .prefix(".pfp-thumb-tmp-")
        .suffix(".jpg")
        .tempfile_in(&dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    tmp.persist(&target).map_err(|e| PhotosError::Io(e.error))?;

    // Best-effort parent fsync. Failure here doesn't invalidate the write.
    if let Ok(dir_fd) = OpenOptions::new().read(true).open(&dir) {
        let _ = dir_fd.sync_all();
    }
    Ok(())
}

pub fn read_cached_thumbnail(folder: &Path, key: &str) -> Result<Vec<u8>, PhotosError> {
    let p = cache_file(folder, key);
    Ok(std::fs::read(&p)?)
}

fn hex_lowercase(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}
