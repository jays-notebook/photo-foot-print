//! Phase 2 -- jpeg.rs unit tests via integration-test seam.

use std::io::Write;
use std::path::Path;
use pfp_photos::jpeg::{has_jpeg_extension, is_real_jpeg};

#[test]
fn extension_lowercase_jpg_accepted() {
    assert!(has_jpeg_extension(Path::new("photo.jpg")));
}

#[test]
fn extension_uppercase_jpeg_accepted() {
    assert!(has_jpeg_extension(Path::new("PHOTO.JPEG")));
}

#[test]
fn extension_mixed_case_jpg_accepted() {
    assert!(has_jpeg_extension(Path::new("Photo.Jpg")));
}

#[test]
fn extension_png_rejected() {
    assert!(!has_jpeg_extension(Path::new("photo.png")));
}

#[test]
fn extension_no_extension_rejected() {
    assert!(!has_jpeg_extension(Path::new("photo")));
}

#[test]
fn extension_near_match_rejected() {
    assert!(!has_jpeg_extension(Path::new("photo.jpgg")));
}

#[test]
fn magic_real_jpeg_accepted() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("good.jpg");
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(&[0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10]).unwrap();
    drop(f);
    assert!(is_real_jpeg(&p));
}

#[test]
fn magic_gif_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("bad.jpg"); // wrong magic, right ext
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(b"GIF89a").unwrap();
    drop(f);
    assert!(!is_real_jpeg(&p));
}

#[test]
fn magic_too_short_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let p = tmp.path().join("short.jpg");
    let mut f = std::fs::File::create(&p).unwrap();
    f.write_all(&[0xFF, 0xD8]).unwrap(); // only 2 bytes
    drop(f);
    assert!(!is_real_jpeg(&p));
}

#[test]
fn magic_nonexistent_rejected() {
    assert!(!is_real_jpeg(Path::new("/nonexistent/path/photo.jpg")));
}
