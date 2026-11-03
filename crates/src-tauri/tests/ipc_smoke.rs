//! Headless coverage of the library operations used by IPC commands.
//! Actual webview/dialog interaction and private scanner metadata gates are separate.

use pfp_exif::{read_detail, read_summary, write_metadata, CaptureTimeChange};
use pfp_photos::{cache, list_folder, thumbnail::decode_resize_encode};

#[path = "../../../tests/support/synthetic_jpeg.rs"]
mod synthetic_jpeg;

#[test]
fn listed_unicode_photo_produces_a_decodable_cached_thumbnail() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    let photo = folder.join("필름 사진 01.JPEG");
    synthetic_jpeg::write_jpeg(&photo, 384, 192);
    let original = std::fs::read(&photo).unwrap();

    let listing = list_folder(&folder).expect("list synthetic folder");
    assert_eq!(listing.items.len(), 1);
    assert_eq!(listing.footer.read_failed, 0);
    let entry = &listing.items[0];
    assert_eq!(entry.file_name, "필름 사진 01.JPEG");
    assert_eq!(entry.absolute_path, photo);
    assert!(!entry.has_gps);
    assert!(entry.capture_time.is_none());
    assert!(listing.thumb_cache_writable);
    assert!(!cache::cache_dir(&folder).exists());

    let thumbnail = decode_resize_encode(&entry.absolute_path).expect("thumbnail");
    cache::write_atomic(&folder, &entry.id, &thumbnail).expect("cache thumbnail");
    let cached = cache::read_cached_thumbnail(&folder, &entry.id).expect("read cached thumbnail");
    assert_eq!(cached, thumbnail);
    let image = image::load_from_memory(&cached).expect("decode cached JPEG");
    assert_eq!((image.width(), image.height()), (256, 128));
    assert_eq!(std::fs::read(&photo).unwrap(), original);
    assert_eq!(list_folder(&folder).unwrap().items.len(), 1);
}

#[test]
fn metadata_save_and_relist_preserve_compressed_image_bytes() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    let photo = folder.join("필름 사진 02.jpg");
    synthetic_jpeg::write_jpeg(&photo, 384, 192);
    let original = std::fs::read(&photo).unwrap();
    let original_scan = jpeg_scan(&original);
    assert!(!read_summary(&photo).unwrap().has_gps);
    assert!(read_detail(&photo).unwrap().capture_time.is_none());

    for (lat, lng, change, expected_time) in [
        (
            37.5,
            127.25,
            CaptureTimeChange::Set("1995:06:07 08:09:10".into()),
            Some("1995:06:07 08:09:10"),
        ),
        (
            -33.875,
            151.125,
            CaptureTimeChange::Keep,
            Some("1995:06:07 08:09:10"),
        ),
        (-34.625, -58.375, CaptureTimeChange::Remove, None),
    ] {
        write_metadata(&photo, lat, lng, None, change).expect("save metadata");
        let detail = read_detail(&photo).expect("read saved metadata");
        let (actual_lat, actual_lng) = detail.gps.expect("saved GPS");
        assert!((actual_lat - lat).abs() < 0.000001);
        assert!((actual_lng - lng).abs() < 0.000001);
        assert_eq!(detail.capture_time.as_deref(), expected_time);

        let listing = list_folder(&folder).expect("relist saved photo");
        assert_eq!(listing.items.len(), 1);
        assert_eq!(listing.footer.read_failed, 0);
        assert!(listing.items[0].has_gps);
        assert_eq!(listing.items[0].capture_time.as_deref(), expected_time);

        let saved = std::fs::read(&photo).unwrap();
        let saved_scan = jpeg_scan(&saved);
        assert!(
            saved_scan == original_scan,
            "compressed scan changed: original {} bytes, saved {} bytes, capture time {expected_time:?}",
            original_scan.len(),
            saved_scan.len()
        );
        let decoded = image::load_from_memory(&saved).expect("decode saved photo");
        assert_eq!((decoded.width(), decoded.height()), (384, 192));
    }
}

/// Parse header segment lengths so a marker-looking byte in metadata cannot
/// be mistaken for the start of the compressed scan. The fixture is baseline JPEG.
/// Keep the complete suffix: this encoder adds no trailer, so new bytes after EOI
/// are also a regression (for example, a missing truncation after shrinking EXIF).
fn jpeg_scan(bytes: &[u8]) -> &[u8] {
    assert!(bytes.starts_with(&[0xff, 0xd8]));
    assert!(bytes.ends_with(&[0xff, 0xd9]));
    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        assert_eq!(bytes[offset], 0xff, "invalid JPEG header marker");
        if bytes[offset + 1] == 0xda {
            return &bytes[offset..];
        }
        let length = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
        assert!(length >= 2 && offset + 2 + length <= bytes.len());
        offset += 2 + length;
    }
    panic!("synthetic JPEG has no compressed scan");
}
