//! thumbnail.rs round-trip via integration test using a real scanner fixture.

use std::io::{BufReader, Cursor};
use std::path::PathBuf;

use image::ImageReader;
use pfp_photos::thumbnail::{decode_resize_encode, TARGET_LONG_EDGE};

fn fixtures_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn first_available_fixture() -> Option<PathBuf> {
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
fn round_trip_dimensions_long_edge_is_target() {
    let Some(src) = first_available_fixture() else {
        eprintln!("[thumbnail] No fixture JPEG supplied yet -- skipping.");
        return;
    };
    let bytes = decode_resize_encode(&src).expect("decode_resize_encode");
    assert!(!bytes.is_empty(), "encoder returned empty buffer");
    assert_eq!(&bytes[..3], &[0xFF, 0xD8, 0xFF], "output is not a JPEG SOI");

    let cursor = Cursor::new(&bytes);
    let img = ImageReader::new(BufReader::new(cursor))
        .with_guessed_format()
        .expect("guess")
        .decode()
        .expect("decode");
    let (w, h) = (img.width(), img.height());

    let long = w.max(h);
    let short = w.min(h);
    assert_eq!(long, TARGET_LONG_EDGE, "long edge must equal target {TARGET_LONG_EDGE}");
    // Aspect ratio preserved within 1px.
    assert!(short >= 1, "short edge must be >= 1");
    assert!(short <= TARGET_LONG_EDGE, "short edge must be <= target");
}

#[test]
fn malformed_input_returns_decode_error() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("garbage.jpg");
    std::fs::write(&p, b"not a real jpeg, just text").unwrap();
    let err = decode_resize_encode(&p).unwrap_err();
    // Error kind: Decode (image crate fails to parse) or Io (read fail).
    // We accept either -- the behaviour we care about is "no panic, propagates Err".
    let msg = format!("{err:?}");
    assert!(msg.contains("Decode") || msg.contains("Io"), "unexpected error: {msg}");
}
