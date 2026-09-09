//! Phase 1 success criterion #4 (EXIF-07): atomic-write fault-injection durability.
//!
//! When a panic fires between fsync-temp and rename, the original target file is
//! byte-identical to its pre-write state.
//!
//! Requires the `fault-injection` Cargo feature (NOT a default feature).
//! Run via:
//!     cargo test -p pfp-exif --test fault_injection --features fault-injection -- --include-ignored
//! or:
//!     make test-fault
//!
//! JPEG tests require local scanner fixtures and fail explicitly if absent. Tests that exercise
//! `pfp_exif::atomic::write_via_temp` directly run unconditionally because they do
//! not require a JPEG parser.

#![cfg(feature = "fault-injection")]

use std::sync::atomic::Ordering;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

use pfp_exif::atomic::fault_inject::PANIC_AFTER_TEMP_WRITE;

/// Process-global mutex serializing every test that touches the
/// `PANIC_AFTER_TEMP_WRITE` static. cargo test runs integration tests in
/// parallel by default, so without this mutex one test's "armed" state can leak
/// into another test's window between arm-and-call. This is the explicit
/// mitigation for threat T-03-06 in the plan's threat model.
fn fault_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
}

mod common;

/// Reset the fault flag in case a previous test panicked without clearing it.
fn reset_fault_flag() {
    PANIC_AFTER_TEMP_WRITE.store(0, Ordering::SeqCst);
}

#[test]
#[ignore = "requires fault-injection feature; run via `make test-fault`"]
fn original_survives_panic_after_temp_write() {
    let _guard = fault_lock();
    let src = common::scanner_fixtures().remove(0);

    // Copy the fixture into a fresh temp dir so the test never mutates the fixture.
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    let pristine = std::fs::read(&src).unwrap();
    std::fs::write(&target, &pristine).unwrap();

    for change in [
        pfp_exif::CaptureTimeChange::Keep,
        pfp_exif::CaptureTimeChange::Set("1990:01:02 03:04:05".into()),
        pfp_exif::CaptureTimeChange::Remove,
    ] {
        reset_fault_flag();
        PANIC_AFTER_TEMP_WRITE.store(1, Ordering::SeqCst);
        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = pfp_exif::write_metadata(&target, 35.6586, 139.7454, None, change);
        }))
        .is_err();
        reset_fault_flag();
        assert!(
            panicked,
            "fault injection must reach the pre-rename checkpoint"
        );
        assert_eq!(
            std::fs::read(&target).unwrap(),
            pristine,
            "the original must survive every capture-time operation"
        );
    }
}

#[test]
#[ignore = "requires fault-injection feature; run via `make test-fault`"]
fn original_survives_writer_callback_error() {
    let _guard = fault_lock();
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    let pristine = std::fs::read(&src).unwrap();
    std::fs::write(&target, &pristine).unwrap();

    reset_fault_flag();

    // Use the bare write_via_temp API to exercise the writer-error branch.
    let r = pfp_exif::atomic::write_via_temp(&target, |_tmp_path| {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test-induced writer error",
        ))
    });
    assert!(r.is_err(), "writer error should propagate");

    let after = std::fs::read(&target).unwrap();
    assert_eq!(
        &after[..],
        &pristine[..],
        "writer-callback error must leave the original byte-identical"
    );
}

#[test]
#[ignore = "requires fault-injection feature; run via `make test-fault`"]
fn happy_path_succeeds_with_durable_write() {
    let _guard = fault_lock();
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &target).unwrap();

    reset_fault_flag();
    PANIC_AFTER_TEMP_WRITE.store(0, Ordering::SeqCst); // explicit: no fault

    pfp_exif::write_gps(&target, 35.6586, 139.7454, None, None).expect("happy-path write_gps");

    // Sanity: read_summary now reports has_gps.
    let summary = pfp_exif::read_summary(&target).expect("read_summary");
    assert!(
        summary.has_gps,
        "expected has_gps=true after happy-path write"
    );

    // No orphan temp files left in the parent dir (parent is `tmp.path()`).
    let orphans: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".pfp-tmp-"))
        .collect();
    assert!(
        orphans.is_empty(),
        "found orphan temp file(s) after happy-path write: {:?}",
        orphans.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}

// ---------------------------------------------------------------------------
// Fixture-free tests that exercise `atomic::write_via_temp` directly. These do
// NOT require a real JPEG -- the API operates on arbitrary bytes via the writer
// callback. They run unconditionally so the RED -> GREEN cycle is observable on
// a fresh clone with no fixtures.
// ---------------------------------------------------------------------------

/// Compose a small "JPEG-like" buffer (SOI + JFIF APP0 + EOI) so the file
/// extension routing in tempfile is not relied upon and the original is
/// realistic-looking. Contents are arbitrary bytes; no parser is invoked.
fn make_pristine_bytes() -> Vec<u8> {
    let mut v = Vec::with_capacity(256);
    v.extend_from_slice(&[0xFF, 0xD8, 0xFF, 0xE0]); // SOI + APP0
    v.extend_from_slice(&[0x00, 0x10, b'J', b'F', b'I', b'F', 0x00]); // length + JFIF\0
    v.extend_from_slice(&[0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00]);
    v.extend_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE]); // payload sentinel
    v.extend_from_slice(&[0xFF, 0xD9]); // EOI
    v
}

/// Direct exercise of the atomic helper: when the writer panics AFTER fsync-temp,
/// the original target on disk must be byte-identical to its pre-call state.
/// This is the load-bearing assertion for EXIF-07 / Phase 1 success criterion #4.
#[test]
fn write_via_temp_panic_at_fault_point_preserves_original() {
    let _guard = fault_lock();
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    let pristine = make_pristine_bytes();
    std::fs::write(&target, &pristine).unwrap();

    reset_fault_flag();
    PANIC_AFTER_TEMP_WRITE.store(1, Ordering::SeqCst);

    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // Writer rewrites the temp with new bytes; the durable body of
        // write_via_temp will then fsync the temp and call the fault hook,
        // which panics because PANIC_AFTER_TEMP_WRITE is armed.
        let _ = pfp_exif::atomic::write_via_temp(&target, |tmp_path| {
            std::fs::write(tmp_path, b"REPLACEMENT-CONTENT-MUST-NEVER-LAND")?;
            Ok(())
        });
    }))
    .is_err();

    reset_fault_flag();

    assert!(
        panicked,
        "fault-injection arming should have panicked write_via_temp"
    );

    let after = std::fs::read(&target).unwrap();
    assert_eq!(
        &after[..],
        &pristine[..],
        "original target must survive panic between fsync-temp and rename (EXIF-07)"
    );
}

/// Happy path through the bare API: writer succeeds, no panic armed, target now
/// contains the new bytes, and no `.pfp-tmp-*` siblings remain in the parent dir.
#[test]
fn write_via_temp_happy_path_replaces_target_and_cleans_up() {
    let _guard = fault_lock();
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    let pristine = make_pristine_bytes();
    std::fs::write(&target, &pristine).unwrap();

    reset_fault_flag();
    PANIC_AFTER_TEMP_WRITE.store(0, Ordering::SeqCst);

    let new_bytes: &[u8] = b"NEW-DURABLE-PAYLOAD";
    pfp_exif::atomic::write_via_temp(&target, |tmp_path| {
        std::fs::write(tmp_path, new_bytes)?;
        Ok(())
    })
    .expect("happy-path write_via_temp");

    let after = std::fs::read(&target).unwrap();
    assert_eq!(
        &after[..],
        new_bytes,
        "after successful write_via_temp the target must contain the new bytes"
    );

    let orphans: Vec<_> = std::fs::read_dir(tmp.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with(".pfp-tmp-"))
        .collect();
    assert!(
        orphans.is_empty(),
        "found orphan temp file(s) after happy-path write_via_temp: {:?}",
        orphans.iter().map(|e| e.file_name()).collect::<Vec<_>>()
    );
}

/// Writer-callback returning Err(io::Error) MUST leave the original file
/// byte-identical and clean up the temp file.
#[test]
fn write_via_temp_writer_error_leaves_original_byte_identical() {
    let _guard = fault_lock();
    let tmp = tempfile::tempdir().unwrap();
    let target = tmp.path().join("photo.jpg");
    let pristine = make_pristine_bytes();
    std::fs::write(&target, &pristine).unwrap();

    reset_fault_flag();

    let r = pfp_exif::atomic::write_via_temp(&target, |tmp_path| {
        // Touch the temp before failing -- mimics a partial write that errs.
        std::fs::write(tmp_path, b"PARTIAL-WRITE-WILL-NOT-LAND")?;
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "test-induced writer error",
        ))
    });
    assert!(r.is_err(), "writer error should propagate");

    let after = std::fs::read(&target).unwrap();
    assert_eq!(
        &after[..],
        &pristine[..],
        "writer-callback error must leave the original byte-identical"
    );
}
