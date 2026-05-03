//! Phase 2 -- list_folder integration test with mixed-content tempdir.

use std::io::Write;
use std::path::PathBuf;

use pfp_photos::list_folder;

fn fixtures_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn first_available_jpeg() -> Option<PathBuf> {
    for vendor in ["scanner", "sony", "canon", "nikon"] {
        let dir = fixtures_root().join(vendor);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"))
                    .unwrap_or(false)
                    && std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) > 1024
                {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[test]
fn mixed_content_folder_filters_and_counts_correctly() {
    let Some(src_jpeg) = first_available_jpeg() else {
        eprintln!("[list_folder] No fixture JPEG supplied yet -- skipping.");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path();

    // 3 valid JPEG copies (different filenames so cache_key differs)
    for name in ["a.jpg", "b.jpg", "c.JPEG"] {
        std::fs::copy(&src_jpeg, folder.join(name)).unwrap();
    }

    // 1 PNG (wrong extension)
    let mut png = std::fs::File::create(folder.join("graphic.png")).unwrap();
    png.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();
    drop(png);

    // 1 truncated .jpg (right ext, wrong magic)
    let mut bad = std::fs::File::create(folder.join("broken.jpg")).unwrap();
    bad.write_all(&[0x00, 0x01, 0x02, 0x03, 0x04]).unwrap();
    drop(bad);

    // 1 hidden .DS_Store (dotfile)
    std::fs::write(folder.join(".DS_Store"), b"junk").unwrap();

    // 1 nested .pfp-thumbs/ subdirectory (silently ignored under WR-02:
    // sub-directories are dropped before counting, regardless of name).
    std::fs::create_dir(folder.join(".pfp-thumbs")).unwrap();

    // 1 nested non-dot subdirectory (e.g. `originals/`). Pre-WR-02 this
    // inflated `non_image_hidden`; post-WR-02 it is silently ignored
    // because sub-directories are not part of the flat-folder JPEG count.
    std::fs::create_dir(folder.join("originals")).unwrap();

    let listing = list_folder(folder).expect("list_folder");

    assert_eq!(listing.items.len(), 3, "expected exactly 3 valid JPEGs");
    assert_eq!(listing.footer.total_jpegs, 3);
    // Hidden (post-WR-02 -- directories are NOT counted):
    //   .DS_Store (dotfile) + graphic.png (wrong ext) + broken.jpg (wrong magic) = 3
    assert_eq!(listing.footer.non_image_hidden, 3, "hidden count");
    assert_eq!(listing.footer.read_failed, 0, "no IO failures expected");

    // Sorted ASC by file_name.
    let names: Vec<String> = listing.items.iter().map(|e| e.file_name.clone()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "items must be sorted ASC by file_name");

    // Each item has a 32-char lowercase hex id.
    for item in &listing.items {
        assert_eq!(item.id.len(), 32, "id length");
        assert!(item.id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert!(item.absolute_path.starts_with(folder));
    }

    // thumb_cache_writable should be true on a tempdir.
    assert!(listing.thumb_cache_writable);
}

#[test]
fn empty_folder_returns_zero_items() {
    let tmp = tempfile::tempdir().unwrap();
    let listing = list_folder(tmp.path()).expect("list_folder");
    assert_eq!(listing.items.len(), 0);
    assert_eq!(listing.footer.total_jpegs, 0);
    assert_eq!(listing.footer.non_image_hidden, 0);
    assert_eq!(listing.footer.read_failed, 0);
    assert!(listing.thumb_cache_writable);
}

/// Strip the EXIF APP1 segment from a JPEG byte stream (`Exif\0\0` payload
/// prefix). Returns a fresh `Vec<u8>` -- the input is not mutated.
///
/// The matching logic mirrors `little_exif/src/jpg.rs::clear_segment`:
/// scan markers `FF E*`, read big-endian length, identify APP1 by the
/// `Exif\0\0` payload prefix, splice that segment out.
fn strip_exif_app1(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() < 4 || bytes[0] != 0xFF || bytes[1] != 0xD8 {
        // Not a JPEG; pass through.
        return bytes.to_vec();
    }
    let mut out = Vec::with_capacity(bytes.len());
    out.extend_from_slice(&bytes[..2]); // SOI
    let mut i = 2;
    while i + 3 < bytes.len() {
        if bytes[i] != 0xFF {
            // Hit the entropy-coded segment; copy the rest verbatim.
            out.extend_from_slice(&bytes[i..]);
            return out;
        }
        let marker = bytes[i + 1];
        // SOS (0xDA) starts the compressed image data; copy from here to EOI.
        if marker == 0xDA {
            out.extend_from_slice(&bytes[i..]);
            return out;
        }
        // Standalone markers without a length payload (none we care about
        // appear before SOS in well-formed JPEGs from scanners).
        let len = u16::from_be_bytes([bytes[i + 2], bytes[i + 3]]) as usize;
        let segment_end = i + 2 + len; // marker (2) + length-bytes-included-in-len
        if segment_end > bytes.len() {
            // Malformed; copy the rest verbatim.
            out.extend_from_slice(&bytes[i..]);
            return out;
        }
        // APP1 EXIF segment? Payload starts with "Exif\0\0".
        if marker == 0xE1
            && segment_end - i >= 10
            && &bytes[i + 4..i + 10] == b"Exif\0\0"
        {
            // Skip this segment entirely.
            i = segment_end;
            continue;
        }
        out.extend_from_slice(&bytes[i..segment_end]);
        i = segment_end;
    }
    if i < bytes.len() {
        out.extend_from_slice(&bytes[i..]);
    }
    out
}

#[test]
fn jpeg_without_exif_segment_appears_in_listing_with_no_gps() {
    // BL-02 regression: a JPEG with no APP1/EXIF segment must NOT be
    // dropped to footer.read_failed; it must appear in items with
    // has_gps:false and capture_time:None.
    let Some(src_jpeg) = first_available_jpeg() else {
        eprintln!("[list_folder] No fixture JPEG -- skipping BL-02 regression.");
        return;
    };
    let original = std::fs::read(&src_jpeg).unwrap();
    let stripped = strip_exif_app1(&original);
    // Sanity: stripping must not have produced an identical buffer (else
    // the source already had no EXIF and the test is degenerate).
    assert_ne!(
        stripped.len(),
        original.len(),
        "fixture has no EXIF to strip -- pick a different fixture"
    );

    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path();
    std::fs::write(folder.join("no-exif.jpg"), &stripped).unwrap();

    let listing = list_folder(folder).expect("list_folder");
    assert_eq!(listing.items.len(), 1, "exif-less JPEG must still be listed");
    assert_eq!(listing.footer.read_failed, 0, "must not be in read_failed");
    let item = &listing.items[0];
    assert_eq!(item.file_name, "no-exif.jpg");
    assert!(!item.has_gps, "no exif => no gps");
    assert!(item.capture_time.is_none(), "no exif => no capture_time");
}

#[test]
fn scanner_fixtures_have_no_gps() {
    // Per D-10: scanner-output JPEGs never have GPS. The list correctly
    // surfaces this through the strict-4 predicate (Plan 02-01).
    let scanner_dir = fixtures_root().join("scanner");
    if !scanner_dir.exists() {
        eprintln!("[list_folder] scanner fixtures dir missing -- skipping.");
        return;
    }
    let listing = list_folder(&scanner_dir).expect("list_folder");
    if listing.items.is_empty() {
        eprintln!("[list_folder] scanner fixtures dir empty -- skipping.");
        return;
    }
    for item in &listing.items {
        assert!(!item.has_gps, "scanner fixture {} unexpectedly has GPS", item.file_name);
    }
}
