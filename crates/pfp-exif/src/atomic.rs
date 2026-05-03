//! Atomic write: temp file in the target's parent dir, then rename. Plan 03
//! upgrades the body to add a media-level flush (Apple's full-fsync fcntl) and
//! a parent-directory fsync.
//!
//! NOTE (Plan 02): this file ships a *stub* body that does NOT call any
//! disk-level flush nor `fsync` on the parent directory. Plan 03 replaces the
//! body with the full durable version (and adds the `fault-injection` Cargo
//! feature plus a fault-injection module).
//!
//! Signature is LOCKED -- Plan 03 does NOT change it. This plan's tests continue
//! to pass after Plan 03 lands.
//!
//! TDD RED (this commit): the `fault_inject` module is declared and the
//! `fault-injection` Cargo feature is registered, but the durable body and the
//! fault hook call site are intentionally absent. Integration tests in
//! `tests/fault_injection.rs` must FAIL: no panic fires, so the assertion
//! `panicked == true` is violated.

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

/// Test-only fault-injection hook for Phase 1 success criterion #4 (EXIF-07).
///
/// The Cargo feature `fault-injection` is NOT a default feature. Production builds
/// (`cargo build`, `cargo build --release`) compile WITHOUT this module entirely.
/// Integration tests opt in via `cargo test --features fault-injection`.
///
/// Why a feature flag and not just `cfg(test)`: integration tests in
/// `crates/pfp-exif/tests/` compile as separate crates that link against the
/// release-flavored `pfp_exif`, where `cfg(test)` is FALSE. A feature makes the
/// hook reachable from there without polluting prod builds.
///
/// TDD RED (this commit): the module is declared but `write_via_temp` does NOT
/// call `maybe_panic_after_temp_write`. The integration test will arm the flag
/// and call `pfp_exif::write_gps`, expecting a panic; the panic does not fire,
/// so the test fails -- the desired RED outcome.
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
