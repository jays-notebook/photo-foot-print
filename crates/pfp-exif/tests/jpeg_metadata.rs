use little_exif::{exif_tag::ExifTag, metadata::Metadata};

fn jpeg() -> Vec<u8> {
    vec![
        0xff, 0xd8, 0xff, 0xe0, 0, 16, b'J', b'F', b'I', b'F', 0, 1, 1, 0, 0, 1, 0, 1, 0, 0, 0xff,
        0xd9,
    ]
}

fn app1(payload: &[u8]) -> Vec<u8> {
    let mut segment = vec![0xff, 0xe1];
    segment.extend(((payload.len() + 2) as u16).to_be_bytes());
    segment.extend(payload);
    segment
}

#[test]
fn exif_less_and_xmp_only_jpegs_can_be_read_and_geotagged() {
    for xmp in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        let mut bytes = jpeg();
        let segment = app1(b"http://ns.adobe.com/xap/1.0/\0<xmp>preserve me</xmp>");
        if xmp {
            bytes.splice(2..2, segment.clone());
        }
        std::fs::write(&path, &bytes).unwrap();
        assert!(!pfp_exif::read_summary(&path).unwrap().has_gps);
        assert!(pfp_exif::read_detail(&path).unwrap().gps.is_none());
        pfp_exif::write_gps(&path, 37.0, 127.0, None, None).unwrap();
        assert_eq!(
            pfp_exif::read_detail(&path).unwrap().gps,
            Some((37.0, 127.0))
        );
        let after = std::fs::read(&path).unwrap();
        if xmp {
            assert!(after.windows(segment.len()).any(|w| w == segment));
        }
        assert!(after.windows(18).any(|w| w == &jpeg()[2..20]));
    }
}

#[test]
fn exif_after_xmp_is_read_and_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    std::fs::write(&path, jpeg()).unwrap();
    let mut metadata = Metadata::new();
    metadata.set_tag(ExifTag::DateTimeOriginal("1990:01:01 00:00:00".into()));
    metadata.set_tag(ExifTag::CreateDate("2026:01:01 00:00:00".into()));
    metadata.write_to_file(&path).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    bytes.splice(2..2, app1(b"http://ns.adobe.com/xap/1.0/\0<xmp/>"));
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(
        pfp_exif::read_detail(&path)
            .unwrap()
            .capture_time
            .as_deref(),
        Some("1990:01:01 00:00:00")
    );
    pfp_exif::write_gps(&path, 37.0, 127.0, None, None).unwrap();
    let after = Metadata::new_from_path(&path).unwrap();
    assert!(
        matches!(after.get_tag(&ExifTag::CreateDate(String::new())).next(), Some(ExifTag::CreateDate(s)) if s == "2026:01:01 00:00:00")
    );
}

#[test]
fn corrupt_exif_and_truncated_jpegs_fail_without_changing_original() {
    for bytes in [vec![0xff, 0xd8, 0xff, 0xe1, 0, 20], {
        let mut bytes = jpeg();
        bytes.splice(2..2, app1(b"Exif\0\0broken"));
        bytes
    }] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        std::fs::write(&path, &bytes).unwrap();
        assert!(pfp_exif::read_detail(&path).is_err());
        assert!(pfp_exif::write_gps(&path, 37.0, 127.0, None, None).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn out_of_segment_ifd_is_rejected_before_reading_or_saving() {
    // little_exif used to treat an absent IFD beyond the APP1 as empty metadata.
    let mut payload = b"Exif\0\0II\x2a\0".to_vec();
    payload.extend(4096_u32.to_le_bytes());
    let mut bytes = jpeg();
    bytes.splice(2..2, app1(&payload));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid-ifd.jpg");
    std::fs::write(&path, &bytes).unwrap();

    assert!(pfp_exif::read_summary(&path).is_err());
    assert!(pfp_exif::read_detail(&path).is_err());
    assert!(pfp_exif::write_gps(&path, 37.0, 127.0, None, None).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn capture_time_keep_set_and_remove_round_trip() {
    use pfp_exif::{write_metadata, CaptureTimeChange};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");
    std::fs::write(&path, jpeg()).unwrap();
    let mut original = Metadata::new();
    original.set_tag(ExifTag::DateTimeOriginal("1990:01:01 00:00:00".into()));
    original.set_tag(ExifTag::CreateDate("2026:01:01 00:00:00".into()));
    original.set_tag(ExifTag::Artist("Preserved artist".into()));
    original.write_to_file(&path).unwrap();
    write_metadata(&path, 37.0, 127.0, None, CaptureTimeChange::Keep).unwrap();
    let kept = Metadata::new_from_path(&path).unwrap();
    assert!(
        matches!(kept.get_tag(&ExifTag::CreateDate(String::new())).next(), Some(ExifTag::CreateDate(s)) if s == "2026:01:01 00:00:00")
    );
    let date = "2001:02:03 04:05:00";
    write_metadata(
        &path,
        37.0,
        127.0,
        None,
        CaptureTimeChange::Set(date.into()),
    )
    .unwrap();
    assert_eq!(
        pfp_exif::read_detail(&path)
            .unwrap()
            .capture_time
            .as_deref(),
        Some(date)
    );
    let set = Metadata::new_from_path(&path).unwrap();
    assert!(
        matches!(set.get_tag(&ExifTag::CreateDate(String::new())).next(), Some(ExifTag::CreateDate(s)) if s == date)
    );
    let before_invalid = std::fs::read(&path).unwrap();
    assert!(write_metadata(
        &path,
        37.0,
        127.0,
        None,
        CaptureTimeChange::Set("invalid".into())
    )
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before_invalid);
    write_metadata(&path, 37.0, 127.0, None, CaptureTimeChange::Remove).unwrap();
    let detail = pfp_exif::read_detail(&path).unwrap();
    assert_eq!(detail.capture_time, None);
    assert_eq!(detail.gps, Some((37.0, 127.0)));
    let removed = Metadata::new_from_path(&path).unwrap();
    assert!(removed
        .get_tag(&ExifTag::CreateDate(String::new()))
        .next()
        .is_none());
    assert!(
        matches!(removed.get_tag(&ExifTag::Artist(String::new())).next(), Some(ExifTag::Artist(s)) if s == "Preserved artist")
    );
}

#[test]
fn oversized_gps_addition_leaves_original_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large-exif.jpg");
    std::fs::write(&path, jpeg()).unwrap();
    let mut metadata = Metadata::new();
    metadata.set_tag(ExifTag::ImageDescription("x".repeat(65_400)));
    metadata.write_to_file(&path).unwrap();
    let before = std::fs::read(&path).unwrap();
    pfp_exif::read_detail(&path).expect("original EXIF is readable");

    let result = pfp_exif::write_gps(&path, 37.0, 127.0, None, None);

    assert!(result.is_err(), "oversized EXIF must be rejected");
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn oversized_capture_time_addition_leaves_original_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large-exif.jpg");
    std::fs::write(&path, jpeg()).unwrap();
    pfp_exif::write_gps(&path, 37.0, 127.0, None, None).unwrap();
    let mut metadata = Metadata::new_from_path(&path).unwrap();
    metadata.set_tag(ExifTag::ImageDescription("x".repeat(65_350)));
    metadata.write_to_file(&path).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert_eq!(
        pfp_exif::read_detail(&path).unwrap().gps,
        Some((37.0, 127.0))
    );

    let result = pfp_exif::write_metadata(
        &path,
        37.0,
        127.0,
        None,
        pfp_exif::CaptureTimeChange::Set("1990:01:02 03:04:05".into()),
    );

    assert!(
        result.is_err(),
        "capture time must not overflow existing EXIF"
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn large_legal_exif_preserves_description_when_adding_gps() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("large-exif.jpg");
    std::fs::write(&path, jpeg()).unwrap();
    let description = "x".repeat(65_350);
    let mut metadata = Metadata::new();
    metadata.set_tag(ExifTag::ImageDescription(description.clone()));
    metadata.write_to_file(&path).unwrap();

    pfp_exif::write_gps(&path, 37.0, 127.0, None, None).unwrap();

    assert_eq!(
        pfp_exif::read_detail(&path).unwrap().gps,
        Some((37.0, 127.0))
    );
    let saved = Metadata::new_from_path(&path).unwrap();
    assert!(
        matches!(saved.get_tag(&ExifTag::ImageDescription(String::new())).next(),
        Some(ExifTag::ImageDescription(s)) if s == &description)
    );
}

#[test]
fn invalid_coordinates_leave_original_byte_identical() {
    for (lat, lng) in [
        (91.0, 127.0),
        (-91.0, 127.0),
        (37.0, 200.0),
        (37.0, -200.0),
        (f64::NAN, 127.0),
        (37.0, f64::NAN),
        (f64::INFINITY, 127.0),
        (37.0, f64::NEG_INFINITY),
        (90.0, 127.0),
        (-90.0, 127.0),
        (37.0, 180.0),
        (37.0, -180.0),
        (0.0, 0.0),
        // These round to reader-invalid values at EXIF rational precision.
        (89.9999999999, 127.0),
        (37.0, 179.9999999999),
        (0.0000000001, 0.0),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("photo.jpg");
        std::fs::write(&path, jpeg()).unwrap();
        pfp_exif::write_gps(&path, 37.0, 127.0, None, None).unwrap();
        let before = std::fs::read(&path).unwrap();

        let result = pfp_exif::write_gps(&path, lat, lng, None, None);

        assert!(
            result.is_err(),
            "invalid GPS ({lat}, {lng}) must be rejected"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
