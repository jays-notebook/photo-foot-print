//! Atomic write: same-directory tempfile -> fsync -> F_FULLFSYNC (macOS) ->
//! rename -> parent-directory fsync.
//!
//! Phase 1 success criterion #4 (EXIF-07): a crash mid-write MUST never leave
//! the original truncated or corrupt. The full pattern below is what
//! `little_exif`'s `write_to_file` is wrapped in by `pfp_exif::write_gps`.
//!
//! Sources:
//! - Apple fcntl(2) manpage: F_FULLFSYNC "asks the drive to flush all buffered
//!   data to the permanent storage device". Plain fsync on macOS does NOT
//!   guarantee media-level flush.
//! - tempfile docs: NamedTempFile::persist does an atomic rename but does NOT
//!   fsync the file or the directory; we wrap it manually.
//! - PITFALLS.md §2: temp file MUST be in the target's parent dir, not the
//!   process-global tmp dir -- cross-volume rename falls back to copy-and-unlink
//!   and is no longer atomic.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
use std::os::unix::io::AsRawFd;

/// Write to a same-directory tempfile via `writer`, then atomically rename it
/// over `target` with full durability:
///
/// 1. Create a sibling NamedTempFile in `target.parent()`.
/// 2. Seed the temp with the original's bytes so callers (e.g.,
///    `pfp_exif::write_gps`) can mutate metadata in place via `little_exif`.
/// 3. Run the caller's `writer` callback, which may rewrite the temp.
/// 4. fsync(temp_fd) -- flush kernel cache to the FS layer.
/// 5. F_FULLFSYNC(temp_fd) on macOS -- ask the drive to flush its own cache.
/// 6. Fault-injection point (`PANIC_AFTER_TEMP_WRITE`) -- exposed only when
///    `cfg(test)` or `feature = "fault-injection"` is set.
/// 7. tempfile.persist(target) -- atomic rename.
/// 8. fsync(parent_dir_fd) -- durably commit the rename.
///
/// Signature is identical to Plan 02's stub.
pub fn write_via_temp<F>(target: &Path, writer: F) -> io::Result<()>
where
    F: FnOnce(&Path) -> io::Result<()>,
{
    let parent = target.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "target has no parent")
    })?;

    // Same-directory tempfile (NOT the process-global tmp dir; see PITFALLS.md §2).
    // Prefix `.pfp-tmp-` makes orphans visible and `.gitignore`-able. The
    // `.jpg` suffix preserves `little_exif`'s extension-based file-type
    // routing inside the writer callback.
    let tmp = tempfile::Builder::new()
        .prefix(".pfp-tmp-")
        .suffix(".jpg")
        .tempfile_in(parent)?;

    // Seed the temp with the original's bytes so the writer callback (which
    // mutates EXIF in place via `little_exif::Metadata::write_to_file`) has a
    // valid JPEG to start from. If `target` does not yet exist (synthesized
    // tests), skip seeding -- the writer must populate the temp itself.
    if target.exists() {
        std::fs::copy(target, tmp.path())?;
    }

    // Run the caller's writer against the same path the temp file occupies.
    // PITFALLS.md §3 (macOS xattrs) -- full xattr preservation is post-v1; documented.
    writer(tmp.path())?;

    // Step 4: fsync the temp FD. Reopening in read mode for the FD because the
    // writer callback may have closed its own File handle.
    {
        let f = File::open(tmp.path())?;
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
        // f is dropped here, closing the read FD. The temp file's bytes are durable.
    }

    // Step 6: fault-injection hook (test-only). The panic fires AFTER the temp
    // file is on durable media but BEFORE the rename, so the test can verify
    // the original target is untouched.
    #[cfg(any(test, feature = "fault-injection"))]
    fault_inject::maybe_panic_after_temp_write();

    // Step 7: atomic rename. tempfile::NamedTempFile::persist consumes the
    // temp handle and does std::fs::rename under the hood.
    tmp.persist(target).map_err(|e| e.error)?;

    // Step 8: fsync the parent directory so the rename is durable. Without
    // this, a power loss after the rename can revert the directory entry on
    // some filesystems even though the temp file's contents are flushed.
    let parent_fd = OpenOptions::new().read(true).open(parent)?;
    parent_fd.sync_all()?;

    Ok(())
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
