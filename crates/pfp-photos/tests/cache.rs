//! cache.rs unit tests via integration-test seam.

use std::path::Path;

use pfp_photos::cache::{
    cache_dir, cache_file, cache_key, ensure_cache_dir, read_cached_thumbnail,
    write_atomic, THUMB_CACHE_DIR,
};
use pfp_photos::error::PhotosError;

#[test]
fn cache_key_is_deterministic() {
    let k1 = cache_key("photo.jpg", 1_700_000_000, 4096);
    let k2 = cache_key("photo.jpg", 1_700_000_000, 4096);
    assert_eq!(k1, k2);
    assert_eq!(k1.len(), 32);
    assert!(k1.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
}

#[test]
fn cache_key_changes_with_filename() {
    let a = cache_key("a.jpg", 1, 1);
    let b = cache_key("b.jpg", 1, 1);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_with_mtime() {
    let a = cache_key("photo.jpg", 1, 1);
    let b = cache_key("photo.jpg", 2, 1);
    assert_ne!(a, b);
}

#[test]
fn cache_key_changes_with_size() {
    let a = cache_key("photo.jpg", 1, 1);
    let b = cache_key("photo.jpg", 1, 2);
    assert_ne!(a, b);
}

#[test]
fn cache_dir_is_dot_pfp_thumbs() {
    let folder = Path::new("/tmp/somewhere");
    assert_eq!(cache_dir(folder), folder.join(THUMB_CACHE_DIR));
    assert_eq!(THUMB_CACHE_DIR, ".pfp-thumbs");
}

#[test]
fn cache_file_appends_jpg() {
    let folder = Path::new("/tmp/somewhere");
    let f = cache_file(folder, "deadbeefcafebabe1234567890abcdef");
    assert!(f.to_string_lossy().ends_with(".pfp-thumbs/deadbeefcafebabe1234567890abcdef.jpg"));
}

#[test]
fn ensure_cache_dir_creates_dir() {
    let tmp = tempfile::tempdir().unwrap();
    ensure_cache_dir(tmp.path()).expect("create");
    assert!(cache_dir(tmp.path()).is_dir());
}

#[test]
fn write_atomic_round_trip() {
    let tmp = tempfile::tempdir().unwrap();
    let key = "abc1234567890def1234567890abcdef";
    let bytes = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10];
    write_atomic(tmp.path(), key, &bytes).expect("write");

    let read_back = read_cached_thumbnail(tmp.path(), key).expect("read");
    assert_eq!(read_back, bytes);
}

#[test]
fn write_atomic_overwrites() {
    let tmp = tempfile::tempdir().unwrap();
    let key = "abc1234567890def1234567890abcdef";

    write_atomic(tmp.path(), key, &[0u8; 4]).expect("write 1");
    write_atomic(tmp.path(), key, &[1u8; 8]).expect("write 2");
    let read_back = read_cached_thumbnail(tmp.path(), key).expect("read");
    assert_eq!(read_back, vec![1u8; 8]);
}

#[test]
fn read_missing_returns_io_error() {
    let tmp = tempfile::tempdir().unwrap();
    let err = read_cached_thumbnail(tmp.path(), "deadbeefcafebabe1234567890abcdef").unwrap_err();
    assert!(matches!(err, PhotosError::Io(_)));
}
