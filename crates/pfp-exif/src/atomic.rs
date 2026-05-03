//! Atomic write: temp file in the target's parent dir, write, fsync, F_FULLFSYNC,
//! rename, parent fsync.
//!
//! NOTE (Plan 02): this file ships a *stub* body that does NOT use `F_FULLFSYNC`
//! and does NOT call `fsync` on the parent directory. Plan 03 replaces the body
//! with the full durable version (and adds the `fault-injection` Cargo feature
//! plus a fault-injection module).
//!
//! Signature is LOCKED -- Plan 03 does NOT change it. This plan's tests continue
//! to pass after Plan 03 lands.

use std::fs;
use std::io;
use std::path::Path;

/// Write to a same-directory tempfile via `writer`, then rename it over `target`.
pub fn write_via_temp<F>(target: &Path, writer: F) -> io::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "target has no parent")
    })?;

    // Sibling temp file in the same dir (NOT std::env::temp_dir -- would cross
    // volume, breaking atomic rename -- see PITFALLS.md §2). The extension must
    // remain ".jpg" so little_exif's extension-based file-type detection routes
    // the writer to the JPEG codepath.
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let tmp_path = parent.join(format!(".pfp-tmp-{pid}-{nanos}.jpg"));

    // Copy the original so the writer (which mutates in place via little_exif) has
    // a valid JPEG to start from.
    fs::copy(target, &tmp_path)?;

    let result = writer(&tmp_path);
    if let Err(e) = result {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }

    fs::rename(&tmp_path, target)?;
    Ok(())
}
