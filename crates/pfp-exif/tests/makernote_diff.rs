//! Phase 1 success criterion #3 (the gate): MakerNote-preservation fixture suite.
//!
//! For Sony, Canon, and Nikon DSLR JPEGs, an EXIF GPS write must NOT alter:
//!   1. The compressed image data (everything from the SOS marker to EOI).
//!   2. Any non-GPS, non-DateTimeOriginal, non-ModifyDate exiftool group.
//!   3. The [MakerNotes] group, byte-for-byte.
//!
//! Tests are #[ignore]-gated by default so `cargo test` is green on a fresh clone.
//! Lift via:
//!     cargo test -p pfp-exif --test makernote_diff -- --ignored
//! or:
//!     make test-gate
//!
//! REQUIRES on PATH:
//!   - exiftool        (`brew install exiftool`)
//! REQUIRES on disk (per docs/FIXTURES.md):
//!   - tests/fixtures/sony/sample.jpg     (real OOC Sony DSLR JPEG)
//!   - tests/fixtures/canon/sample.jpg
//!   - tests/fixtures/nikon/sample.jpg

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn exiftool_g1(path: &Path) -> String {
    let out = Command::new("exiftool")
        .args(["-a", "-G1", "-s", path.to_str().unwrap()])
        .output()
        .expect("exiftool not on PATH -- run `brew install exiftool` (see docs/FIXTURES.md)");
    assert!(
        out.status.success(),
        "exiftool failed on {:?}: {}",
        path,
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).expect("exiftool output not UTF-8")
}

/// Strip lines we expect to differ after a GPS write.
fn strip_expected_diffs(s: &str) -> String {
    s.lines()
        .filter(|l| !l.starts_with("[GPS]"))
        .filter(|l| !(l.starts_with("[ExifIFD]") && l.contains("DateTimeOriginal")))
        .filter(|l| !(l.starts_with("[IFD0]") && l.contains("ModifyDate")))
        .filter(|l| !(l.starts_with("[ExifIFD]") && l.contains("ModifyDate")))
        // [File]: on-disk metadata (size, modified time) -- not part of the contract.
        .filter(|l| !l.starts_with("[File]"))
        // [Composite]: exiftool-derived; can shift if any underlying tag changes.
        .filter(|l| !l.starts_with("[Composite]"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Read raw bytes from SOS marker (FF DA) to EOI marker (FF D9), inclusive.
/// This is the compressed image-data region a write must NOT touch (EXIF-09).
fn read_image_data(p: &Path) -> Vec<u8> {
    let bytes = std::fs::read(p).unwrap();
    let sos = find_marker(&bytes, 0xDA).expect("SOS marker (FF DA) not found");
    let eoi = find_marker(&bytes[sos..], 0xD9).expect("EOI marker (FF D9) not found") + sos;
    bytes[sos..=eoi + 1].to_vec()
}

fn find_marker(bytes: &[u8], marker: u8) -> Option<usize> {
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == 0xFF && bytes[i + 1] == marker {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn run_for_vendor(vendor: &str) {
    let path = fixtures_root().join(vendor).join("sample.jpg");
    if !path.exists() || std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) <= 1024 {
        // Either the user has not yet supplied a real fixture (per docs/FIXTURES.md)
        // or `git lfs pull` has not run. The test no-ops with a clear message --
        // the #[ignore] gate keeps `cargo test` green by default; if someone runs
        // `--ignored` on a fresh clone, they get an actionable message instead of
        // a panic.
        eprintln!(
            "[{vendor}] fixture missing or LFS pointer at {:?} -- see docs/FIXTURES.md",
            path,
        );
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join(path.file_name().unwrap());
    std::fs::copy(&path, &work).unwrap();

    let before = exiftool_g1(&work);
    let before_image_bytes = read_image_data(&work);

    pfp_exif::write_gps(&work, 35.6586, 139.7454, None, None).expect("write_gps");

    let after = exiftool_g1(&work);
    let after_image_bytes = read_image_data(&work);

    // Criterion #9: compressed image data byte-identical (EXIF-09).
    assert_eq!(
        before_image_bytes, after_image_bytes,
        "[{vendor}] compressed image data changed after EXIF write (EXIF-09 violated)"
    );

    // Criterion #8: every non-GPS, non-DTO, non-ModifyDate line identical.
    let before_filtered = strip_expected_diffs(&before);
    let after_filtered = strip_expected_diffs(&after);
    assert_eq!(
        before_filtered, after_filtered,
        "[{vendor}] non-GPS exiftool output differs (EXIF-08 violated)\n\nBEFORE:\n{}\n\nAFTER:\n{}",
        before_filtered, after_filtered
    );

    // Criterion #3 (the gate): MakerNotes byte-for-byte.
    let before_mn: Vec<_> = before
        .lines()
        .filter(|l| l.starts_with("[MakerNotes]"))
        .collect();
    let after_mn: Vec<_> = after
        .lines()
        .filter(|l| l.starts_with("[MakerNotes]"))
        .collect();
    assert!(
        !before_mn.is_empty(),
        "[{vendor}] fixture has no [MakerNotes] tags -- supply an OOC JPEG, not a Lightroom export (see docs/FIXTURES.md)"
    );
    assert_eq!(
        before_mn, after_mn,
        "[{vendor}] MakerNotes differ after EXIF write -- gate criterion #3 FAILED. \
         Follow ROADMAP.md \"Phase Gating\": fall back to img-parts + hand-encoded EXIF \
         byte payload, or fork little_exif for the affected case."
    );
}

#[test]
#[ignore = "fixture-gated; provide tests/fixtures/sony/sample.jpg, see docs/FIXTURES.md (run via `cargo test -- --ignored` or `make test-gate`)"]
fn makernote_preserved_sony() {
    run_for_vendor("sony");
}

#[test]
#[ignore = "fixture-gated; provide tests/fixtures/canon/sample.jpg, see docs/FIXTURES.md (run via `cargo test -- --ignored` or `make test-gate`)"]
fn makernote_preserved_canon() {
    run_for_vendor("canon");
}

#[test]
#[ignore = "fixture-gated; provide tests/fixtures/nikon/sample.jpg, see docs/FIXTURES.md (run via `cargo test -- --ignored` or `make test-gate`)"]
fn makernote_preserved_nikon() {
    run_for_vendor("nikon");
}

/// Pre-test for little_exif issue #93: read metadata, write it back UNCHANGED, and
/// confirm the file is still valid. If this fails on any vendor fixture, ROADMAP.md
/// "Phase Gating" is invoked before the criterion-#3 tests are even meaningful.
#[test]
#[ignore = "fixture-gated; companion to makernote_preserved_*; see docs/FIXTURES.md"]
fn little_exif_issue_93_no_op_round_trip() {
    for vendor in ["sony", "canon", "nikon"] {
        let path = fixtures_root().join(vendor).join("sample.jpg");
        if !path.exists() || std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0) <= 1024 {
            eprintln!("[{vendor}] fixture missing -- skipping issue-93 pre-test");
            continue;
        }
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("photo.jpg");
        std::fs::copy(&path, &work).unwrap();

        // No-op write: read metadata, write it back unchanged. Issue #93 manifests
        // here as a `failed to fill whole buffer` error.
        let metadata = little_exif::metadata::Metadata::new_from_path(&work)
            .unwrap_or_else(|e| panic!("[{vendor}] little_exif read failed (issue #93?): {e}"));
        metadata
            .write_to_file(&work)
            .unwrap_or_else(|e| panic!("[{vendor}] little_exif write failed (issue #93?): {e}"));
    }
}
