//! Phase 1 success criterion #3 (the gate): metadata-preservation fixture suite.
//!
//! For representative scanner-output JPEGs (NORITSU / Photoshop pipeline), an
//! EXIF GPS write must NOT alter:
//!   1. The compressed image data (everything from the SOS marker to EOI).
//!      (EXIF-09)
//!   2. Any non-GPS, non-DateTimeOriginal, non-ModifyDate exiftool group --
//!      including ICC profile, XMP, IPTC, Adobe APP14, and Photoshop tags.
//!      (EXIF-08)
//!
//! Per CONTEXT D-10, vendor-specific DSLR MakerNote preservation is **not**
//! checked: this app's target user is a film photographer, source files are
//! scanner output, and scanner JPEGs carry zero MakerNote tags by definition.
//!
//! Tests are #[ignore]-gated by default so `cargo test` is green on a fresh
//! clone. Lift via:
//!     cargo test -p pfp-exif --test metadata_preservation -- --ignored
//! or:
//!     make test-gate
//!
//! REQUIRES on PATH:
//!
//!   - exiftool        (`brew install exiftool`)
//!
//! REQUIRES on disk (per docs/FIXTURES.md):
//!
//!   - tests/fixtures/scanner/*.jpg  (at least one representative file)

use std::path::Path;
use std::process::Command;

mod common;
use common::scanner_fixtures as discover_fixtures;

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
///
/// Expected-drift groups:
/// - `[GPS]`               -- this is what we wrote.
/// - DateTimeOriginal      -- may be created/updated by the writer.
/// - ModifyDate            -- exiftool group hosting it varies.
/// - exiftool's filesystem-metadata groups (`[File]`, `[System]`)
///   -- on-disk mtime / inode-change-time / file size; not part of the contract.
/// - `[IFD1]` ThumbnailOffset -- IFD rewrite shifts the pointer to the same data.
/// - `[Composite]`         -- exiftool-derived; can shift if any underlying tag changes.
fn strip_expected_diffs(s: &str) -> String {
    s.lines()
        .filter(|l| !l.starts_with("[GPS]"))
        .filter(|l| !(l.starts_with("[IFD0]") && l.contains("ModifyDate")))
        .filter(|l| !(l.starts_with("[ExifIFD]") && l.contains("ModifyDate")))
        .filter(|l| !l.starts_with("[File]"))
        .filter(|l| !l.starts_with("[System]"))
        .filter(|l| !(l.starts_with("[IFD1]") && l.contains("ThumbnailOffset")))
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

fn check_one(path: &Path) {
    let label = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");

    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join(path.file_name().unwrap());
    std::fs::copy(path, &work).unwrap();

    let before = exiftool_g1(&work);
    let before_image_bytes = read_image_data(&work);

    pfp_exif::write_gps(&work, 35.6586, 139.7454, None, None).expect("write_gps");

    let after = exiftool_g1(&work);
    let after_image_bytes = read_image_data(&work);

    // EXIF-09: compressed image data byte-identical.
    assert_eq!(
        before_image_bytes, after_image_bytes,
        "[{label}] compressed image data changed after EXIF write (EXIF-09 violated)"
    );

    // EXIF-08: every non-expected-drift line identical.
    let before_filtered = strip_expected_diffs(&before);
    let after_filtered = strip_expected_diffs(&after);
    assert_eq!(
        before_filtered, after_filtered,
        "[{label}] non-GPS exiftool output differs (EXIF-08 violated). \
         If a new exiftool group is appearing in the diff, decide whether it is \
         expected drift (add to strip_expected_diffs) or a real preservation \
         violation (ROADMAP.md \"Phase Gating\" -- fall back to img-parts + \
         hand-encoded EXIF byte payload).\n\nBEFORE:\n{}\n\nAFTER:\n{}",
        before_filtered, after_filtered
    );
}

#[test]
#[ignore = "fixture-gated; provide tests/fixtures/scanner/*.jpg, see docs/FIXTURES.md (run via `cargo test -- --ignored` or `make test-gate`)"]
fn metadata_preserved_across_scanner_fixtures() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "no scanner fixtures found at tests/fixtures/scanner/*.jpg. \
         Phase 1 gate requires at least one representative scanner-output JPEG \
         (see docs/FIXTURES.md). If you didn't intend to run the gate, omit \
         --include-ignored / --ignored or run plain `cargo test`.",
    );
    eprintln!(
        "[scanner] running metadata-preservation gate against {} fixture(s)",
        fixtures.len()
    );
    for path in &fixtures {
        eprintln!("[scanner] checking {:?}", path);
        check_one(path);
    }
}

/// Pre-test for little_exif issue #93: read metadata, write it back UNCHANGED, and
/// confirm the file is still valid. If this fails on any fixture, ROADMAP.md
/// "Phase Gating" is invoked before the metadata-preservation test is even
/// meaningful.
#[test]
#[ignore = "fixture-gated; companion to metadata_preserved_across_scanner_fixtures; see docs/FIXTURES.md"]
fn little_exif_issue_93_no_op_round_trip() {
    let fixtures = discover_fixtures();
    assert!(
        !fixtures.is_empty(),
        "issue-93 pre-test found zero scanner fixtures -- supply at least one \
         JPEG at tests/fixtures/scanner/*.jpg (see docs/FIXTURES.md). \
         If you didn't intend to run the gate, omit --include-ignored / --ignored.",
    );
    for path in &fixtures {
        let label = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        let tmp = tempfile::tempdir().unwrap();
        let work = tmp.path().join("photo.jpg");
        std::fs::copy(path, &work).unwrap();

        // No-op write: read metadata, write it back unchanged. Issue #93 manifests
        // here as a `failed to fill whole buffer` error.
        let metadata = little_exif::metadata::Metadata::new_from_path(&work)
            .unwrap_or_else(|e| panic!("[{label}] little_exif read failed (issue #93?): {e}"));
        metadata
            .write_to_file(&work)
            .unwrap_or_else(|e| panic!("[{label}] little_exif write failed (issue #93?): {e}"));
    }
}

#[test]
#[ignore = "requires local scanner fixtures and ExifTool; run make test-gate"]
fn capture_time_edits_preserve_unrelated_metadata_and_image_bytes() {
    use pfp_exif::{write_metadata, CaptureTimeChange};
    fn without_capture_time(text: &str) -> String {
        strip_expected_diffs(text)
            .lines()
            .filter(|line| {
                !(line.starts_with("[ExifIFD]")
                    && (line.contains("DateTimeOriginal") || line.contains("CreateDate")))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    for path in discover_fixtures() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("photo.jpg");
        std::fs::copy(path, &work).unwrap();
        let before = without_capture_time(&exiftool_g1(&work));
        let image = read_image_data(&work);
        for change in [
            CaptureTimeChange::Set("1990:01:02 03:04:05".into()),
            CaptureTimeChange::Remove,
        ] {
            write_metadata(&work, 37.0, 127.0, None, change).unwrap();
            assert_eq!(without_capture_time(&exiftool_g1(&work)), before);
            assert_eq!(read_image_data(&work), image);
        }
        assert!(pfp_exif::read_detail(&work).unwrap().capture_time.is_none());
    }
}

#[test]
#[ignore = "requires local scanner fixtures and ExifTool; run make test-gate"]
fn scanner_without_exif_preserves_xmp_and_compressed_image() {
    for path in discover_fixtures() {
        let dir = tempfile::tempdir().unwrap();
        let work = dir.path().join("photo.jpg");
        std::fs::copy(path, &work).unwrap();
        let output = Command::new("exiftool")
            .args(["-exif:all=", "-overwrite_original"])
            .arg(&work)
            .output()
            .expect("ExifTool required");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(pfp_exif::read_detail(&work).unwrap().gps.is_none());
        check_one(&work);
    }
}
