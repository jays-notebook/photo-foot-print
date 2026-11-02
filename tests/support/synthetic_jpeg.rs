//! Deterministic, non-photographic JPEGs for portable integration tests.
//! These never read or populate the private scanner-fixture directory.

use std::path::{Path, PathBuf};

use image::{codecs::jpeg::JpegEncoder, Rgb, RgbImage};

pub fn photo_folder() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().expect("temporary fixture root");
    let folder = temp.path().join("사진 테스트 with spaces");
    std::fs::create_dir(&folder).expect("Unicode fixture folder");
    let folder = folder.canonicalize().expect("canonical fixture folder");
    (temp, folder)
}

pub fn write_jpeg(path: &Path, width: u32, height: u32) {
    let pixels = RgbImage::from_fn(width, height, |x, y| {
        Rgb([
            ((x * 7 + y * 3) % 256) as u8,
            ((x * 2 + y * 11) % 256) as u8,
            ((x * 13 + y * 5) % 256) as u8,
        ])
    });
    let file = std::fs::File::create(path).expect("create synthetic JPEG");
    JpegEncoder::new_with_quality(file, 90)
        .encode_image(&pixels)
        .expect("encode synthetic JPEG");
}
