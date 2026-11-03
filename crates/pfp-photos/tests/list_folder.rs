//! Folder listing with real synthetic JPEGs; no private photos required by default.

use std::io::Write;

use pfp_photos::list_folder;

#[path = "../../../tests/support/synthetic_jpeg.rs"]
mod synthetic_jpeg;

#[test]
fn mixed_content_folder_filters_and_counts_correctly() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();

    // Unicode names, spaces, and mixed-case JPEG extensions remain usable.
    for name in ["a photo.jpg", "b.jpg", "촬영 01.JPEG"] {
        synthetic_jpeg::write_jpeg(&folder.join(name), 384, 192);
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

    let listing = list_folder(&folder).expect("list_folder");

    assert_eq!(listing.items.len(), 3, "expected exactly 3 valid JPEGs");
    assert_eq!(listing.footer.total_jpegs, 3);
    // Hidden (post-WR-02 -- directories are NOT counted):
    //   .DS_Store (dotfile) + graphic.png (wrong ext) + broken.jpg (wrong magic) = 3
    assert_eq!(listing.footer.non_image_hidden, 3, "hidden count");
    assert_eq!(listing.footer.read_failed, 0, "no IO failures expected");

    // Sorted ASC by file_name.
    let names: Vec<String> = listing.items.iter().map(|e| e.file_name.clone()).collect();
    assert_eq!(names, ["a photo.jpg", "b.jpg", "촬영 01.JPEG"]);

    // Each item has a 32-char lowercase hex id.
    for item in &listing.items {
        assert_eq!(item.id.len(), 32, "id length");
        assert!(item
            .id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert_eq!(item.absolute_path, folder.join(&item.file_name));
        assert!(!item.has_gps);
        assert!(item.capture_time.is_none());
    }

    // thumb_cache_writable should be true on a tempdir.
    assert!(listing.thumb_cache_writable);
}

#[test]
fn empty_folder_returns_zero_items_without_creating_cache() {
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    let listing = list_folder(&folder).expect("list_folder");
    assert_eq!(listing.items.len(), 0);
    assert_eq!(listing.footer.total_jpegs, 0);
    assert_eq!(listing.footer.non_image_hidden, 0);
    assert_eq!(listing.footer.read_failed, 0);
    assert!(listing.thumb_cache_writable);
    assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
}

#[test]
fn jpeg_without_exif_segment_appears_in_listing_with_no_gps() {
    // BL-02 regression: a JPEG with no APP1/EXIF segment must NOT be
    // dropped to footer.read_failed; it must appear in items with
    // has_gps:false and capture_time:None.
    let (_temp, folder) = synthetic_jpeg::photo_folder();
    // The JPEG encoder creates no EXIF; no private photo needs stripping.
    synthetic_jpeg::write_jpeg(&folder.join("메타데이터 없음.jpg"), 384, 192);

    let listing = list_folder(&folder).expect("list_folder");
    assert_eq!(
        listing.items.len(),
        1,
        "exif-less JPEG must still be listed"
    );
    assert_eq!(listing.footer.read_failed, 0, "must not be in read_failed");
    let item = &listing.items[0];
    assert_eq!(item.file_name, "메타데이터 없음.jpg");
    assert!(!item.has_gps, "no exif => no gps");
    assert!(item.capture_time.is_none(), "no exif => no capture_time");
}

#[test]
#[ignore = "requires local scanner fixtures; see docs/FIXTURES.md"]
fn scanner_fixtures_have_no_gps() {
    let scanner_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/scanner");
    let listing =
        list_folder(&scanner_dir).expect("Supply local scanner fixtures per docs/FIXTURES.md");
    assert!(
        !listing.items.is_empty(),
        "No usable scanner JPEG fixtures; see docs/FIXTURES.md"
    );
    for item in &listing.items {
        assert!(
            !item.has_gps,
            "scanner fixture {} unexpectedly has GPS",
            item.file_name
        );
    }
}
