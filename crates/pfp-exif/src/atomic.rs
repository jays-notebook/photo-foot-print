//! Atomic write: same-directory tempfile -> file flush -> platform replacement.
//! macOS adds F_FULLFSYNC; Unix syncs the parent directory after replacement.
//! Windows uses MoveFileExW with REPLACE_EXISTING and WRITE_THROUGH.
//!
//! Phase 1 success criterion #4 (EXIF-07): a crash mid-write MUST never leave
//! the original truncated or corrupt. `pfp_exif::write_metadata` uses this
//! helper around its JPEG metadata rewrite and readback validation.
//!
//! Sources:
//! - Apple fcntl(2) manpage: F_FULLFSYNC "asks the drive to flush all buffered
//!   data to the permanent storage device". Plain fsync on macOS does NOT
//!   guarantee media-level flush.
//! - tempfile docs: NamedTempFile::persist does an atomic rename but does NOT
//!   fsync the file or the directory; we wrap it manually.
//! - MoveFileExW: WRITE_THROUGH waits for the move to complete on disk. We do
//!   not enable COPY_ALLOWED: a cross-volume copy/delete is not an atomic move.
//!   Actual power-loss durability still depends on the filesystem and device.

use std::fs::OpenOptions;
use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;

/// Write to a same-directory tempfile via `writer`, then atomically rename it
/// over `target`, flushing the file before committing the replacement:
///
/// 1. Create a sibling NamedTempFile in `target.parent()`.
/// 2. Seed the temp with the original's bytes so callers (e.g.,
///    `pfp_exif::write_gps`) can mutate metadata in place via `little_exif`.
/// 3. Run the caller's `writer` callback, which may rewrite the temp.
/// 4. fsync(temp_fd) -- flush kernel cache to the FS layer.
/// 5. F_FULLFSYNC(temp_fd) on macOS -- ask the drive to flush its own cache.
/// 6. Fault-injection point (`PANIC_AFTER_TEMP_WRITE`) -- exposed only when
///    `cfg(test)` or `feature = "fault-injection"` is set.
/// 7. Atomic replacement: tempfile.persist on Unix, write-through move on Windows.
/// 8. On Unix, fsync(parent_dir_fd) to commit the directory entry.
pub fn write_via_temp<F>(target: &Path, writer: F) -> io::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let parent = target
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target has no parent"))?;

    // Same-directory tempfile (NOT the process-global tmp dir; see PITFALLS.md §2).
    // Prefix `.pfp-tmp-` makes orphans visible and `.gitignore`-able. The
    // `.jpg` suffix preserves `little_exif`'s extension-based file-type
    // routing inside the writer callback.
    let tmp = tempfile::Builder::new()
        .prefix(".pfp-tmp-")
        .suffix(".jpg")
        .tempfile_in(parent)?;

    // Seed the temp with the original's bytes so the JPEG metadata rewrite
    // has a valid JPEG to start from. If `target` does not yet exist (synthesized
    // tests), skip seeding -- the writer must populate the temp itself.
    if target.exists() {
        std::fs::copy(target, tmp.path())?;
    }

    // Temporarily clear attributes on the private Windows copy so callbacks
    // using CREATE_ALWAYS can rewrite hidden/system files. Keep the copied
    // attributes for restoration; a read-only original stays protected.
    #[cfg(windows)]
    let copied_attributes = windows::prepare_tempfile(tmp.path())?;

    // Run the caller's writer against the same path the temp file occupies.
    // PITFALLS.md §3 (macOS xattrs) -- full xattr preservation is post-v1; documented.
    writer(tmp.path())?;

    // Restore attributes before flushing, except READONLY (which would prevent
    // cleanup on failure) and TEMPORARY (which may suppress disk writes).
    #[cfg(windows)]
    windows::restore_tempfile_attributes(tmp.path(), copied_attributes)?;

    // Reopen the final path because the callback may replace the temporary file.
    // Windows FlushFileBuffers requires a handle with write access.
    {
        let f = OpenOptions::new().read(true).write(true).open(tmp.path())?;
        f.sync_all()?;

        // Step 5: macOS-strength durability -- F_FULLFSYNC asks the drive to
        // flush its internal buffers (Apple fcntl(2) manpage). Without this,
        // fsync on macOS can return success while bytes still sit in the drive
        // cache.
        #[cfg(target_os = "macos")]
        {
            // SAFETY: f is a valid open FD for the duration of this scope.
            let res = unsafe { libc::fcntl(f.as_raw_fd(), libc::F_FULLFSYNC) };
            if res == -1 {
                return Err(io::Error::last_os_error());
            }
        }
        // Drop this handle before attempting to replace the original.
    }

    // Step 6: fault-injection hook (test-only). The panic fires AFTER the temp
    // file flush has succeeded but BEFORE the rename, so the test can verify
    // the original target is untouched.
    #[cfg(any(test, feature = "fault-injection"))]
    fault_inject::maybe_panic_after_temp_write();

    #[cfg(windows)]
    windows::persist(tmp, target)?;
    #[cfg(not(windows))]
    tmp.persist(target).map_err(|e| e.error)?;

    // Windows does not support this Unix directory-open/fsync sequence; its
    // replacement above requests write-through instead. Keep Unix errors visible.
    #[cfg(unix)]
    {
        let parent_fd = OpenOptions::new().read(true).open(parent)?;
        parent_fd.sync_all()?;
    }

    Ok(())
}

#[cfg(windows)]
mod windows {
    use std::io;
    use std::os::windows::ffi::OsStrExt;
    use std::path::Path;

    use windows_sys::Win32::Storage::FileSystem::{
        GetFileAttributesW, MoveFileExW, SetFileAttributesW, FILE_ATTRIBUTE_NORMAL,
        FILE_ATTRIBUTE_READONLY, FILE_ATTRIBUTE_TEMPORARY, INVALID_FILE_ATTRIBUTES,
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    fn wide_path(path: &Path) -> io::Result<Vec<u16>> {
        let mut wide: Vec<_> = path.as_os_str().encode_wide().collect();
        if wide.contains(&0) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "file paths cannot contain NUL characters",
            ));
        }
        wide.push(0);
        Ok(wide)
    }

    pub(super) fn prepare_tempfile(path: &Path) -> io::Result<u32> {
        let path = wide_path(path)?;
        // SAFETY: path is a live, NUL-terminated UTF-16 string.
        let attributes = unsafe { GetFileAttributesW(path.as_ptr()) };
        if attributes == INVALID_FILE_ATTRIBUTES {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: path remains live and NUL-terminated.
        if unsafe { SetFileAttributesW(path.as_ptr(), FILE_ATTRIBUTE_NORMAL) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(attributes)
    }

    pub(super) fn restore_tempfile_attributes(path: &Path, copied: u32) -> io::Result<()> {
        let path = wide_path(path)?;
        let retained =
            copied & !(FILE_ATTRIBUTE_READONLY | FILE_ATTRIBUTE_TEMPORARY | FILE_ATTRIBUTE_NORMAL);
        // NORMAL is valid only when there are no other attributes.
        let attributes = if retained == 0 {
            FILE_ATTRIBUTE_NORMAL
        } else {
            retained
        };
        // SAFETY: path is a live, NUL-terminated UTF-16 string.
        if unsafe { SetFileAttributesW(path.as_ptr(), attributes) } == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    pub(super) fn persist(mut tmp: tempfile::NamedTempFile, target: &Path) -> io::Result<()> {
        let source = wide_path(tmp.path())?;
        let target = wide_path(target)?;
        // SAFETY: both paths remain live and NUL-terminated for this call.
        // Sibling paths keep the move on one volume. COPY_ALLOWED is omitted.
        if unsafe {
            MoveFileExW(
                source.as_ptr(),
                target.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            // tmp still owns the old path, so failure removes only the copy.
            return Err(io::Error::last_os_error());
        }
        // The source name no longer belongs to us. Retire cleanup only after
        // successful replacement, then let the writable file handle close.
        tmp.disable_cleanup(true);
        Ok(())
    }
}

/// Test-only fault-injection hook for Phase 1 success criterion #4 (EXIF-07).
///
/// The Cargo feature `fault-injection` is NOT a default feature. Production
/// builds (`cargo build`, `cargo build --release`) compile WITHOUT this module
/// entirely. Integration tests opt in via `cargo test --features fault-injection`.
///
/// Why a feature flag and not just `cfg(test)`: integration tests in
/// `crates/pfp-exif/tests/` compile as separate crates that link against the
/// release-flavored `pfp_exif`, where `cfg(test)` is FALSE. A feature makes the
/// hook reachable from there without polluting prod builds.
#[cfg(any(test, feature = "fault-injection"))]
pub mod fault_inject {
    use std::sync::atomic::{AtomicU8, Ordering};

    /// Set to non-zero to make the next call to `maybe_panic_after_temp_write`
    /// panic. Tests use SeqCst for ordering simplicity; this is not perf-critical.
    pub static PANIC_AFTER_TEMP_WRITE: AtomicU8 = AtomicU8::new(0);

    /// Called from inside `write_via_temp` between fsync-temp and rename. Panics
    /// if `PANIC_AFTER_TEMP_WRITE` is set.
    pub fn maybe_panic_after_temp_write() {
        if PANIC_AFTER_TEMP_WRITE.load(Ordering::SeqCst) != 0 {
            panic!("fault-injection: panic after temp write, before rename");
        }
    }
}
