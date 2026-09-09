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
