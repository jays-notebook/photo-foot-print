//! Portable thumbnail decoding and resizing with actual synthetic JPEGs.

use pfp_photos::error::PhotosError;
use pfp_photos::thumbnail::decode_resize_encode;

#[path = "../../../tests/support/synthetic_jpeg.rs"]
mod synthetic_jpeg;

#[test]
fn landscape_and_portrait_thumbnails_preserve_aspect_ratio() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    for (name, source_size, expected_size) in [
        ("가로 사진.jpg", (384, 192), (256, 128)),
        ("세로 사진.JPEG", (192, 384), (128, 256)),
    ] {
        let source = folder.join(name);
        synthetic_jpeg::write_jpeg(&source, source_size.0, source_size.1);
        let before = std::fs::read(&source).unwrap();
        let bytes = decode_resize_encode(&source).expect("decode_resize_encode");
        let thumbnail = image::load_from_memory(&bytes).expect("decode thumbnail JPEG");
        assert_eq!((thumbnail.width(), thumbnail.height()), expected_size);
        assert_eq!(std::fs::read(&source).unwrap(), before, "source changed");
    }
}

#[test]
fn malformed_input_returns_decode_error() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    let path = folder.join("잘못된 사진.jpg");
    std::fs::write(&path, b"not a real jpeg, just text").unwrap();
    assert!(matches!(
        decode_resize_encode(&path),
        Err(PhotosError::Decode(_))
    ));
}
