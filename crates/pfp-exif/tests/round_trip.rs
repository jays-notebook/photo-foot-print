//! Phase 1 success criterion #5: round-trip GPS reads back what we wrote, in all
//! four hemispheres, within +/- 1e-6 degrees.
//!
//! Uses local scanner JPEGs. Run explicitly with `make test-gate`.
//! Missing fixtures fail the gate instead of silently passing.

mod common;

fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
    (a - b).abs() < eps
}

/// Read GPS lat/lng back via the cross-check reader (kamadak-exif).
fn read_back_gps(p: &std::path::Path) -> (f64, f64, char, char) {
    let file = std::fs::File::open(p).unwrap();
    let mut bufreader = std::io::BufReader::new(&file);
    let exif_reader = exif::Reader::new();
    let exif = exif_reader
        .read_from_container(&mut bufreader)
        .expect("kamadak-exif read");

    fn rationals_to_decimal(field: &exif::Field) -> f64 {
        match &field.value {
            exif::Value::Rational(rs) if rs.len() == 3 => {
                rs[0].to_f64() + rs[1].to_f64() / 60.0 + rs[2].to_f64() / 3600.0
            }
            other => panic!("expected rational triple, got {other:?}"),
        }
    }

    fn ref_char(field: &exif::Field) -> char {
        match &field.value {
            exif::Value::Ascii(v) if !v.is_empty() && !v[0].is_empty() => v[0][0] as char,
            other => panic!("expected ascii ref, got {other:?}"),
        }
    }

    let lat_field = exif
        .get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY)
        .expect("GPSLatitude");
    let lng_field = exif
        .get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
        .expect("GPSLongitude");
    let lat_ref_f = exif
        .get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
        .expect("GPSLatitudeRef");
    let lng_ref_f = exif
        .get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
        .expect("GPSLongitudeRef");

    (
        rationals_to_decimal(lat_field),
        rationals_to_decimal(lng_field),
        ref_char(lat_ref_f),
        ref_char(lng_ref_f),
    )
}

fn round_trip_at(name: &str, lat: f64, lng: f64, expected_lat_ref: char, expected_lng_ref: char) {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();

    pfp_exif::write_gps(&work, lat, lng, None, None).expect("write_gps");

    let (read_lat, read_lng, read_lat_ref, read_lng_ref) = read_back_gps(&work);
    let signed_lat = if read_lat_ref == 'S' {
        -read_lat
    } else {
        read_lat
    };
    let signed_lng = if read_lng_ref == 'W' {
        -read_lng
    } else {
        read_lng
    };

    assert!(
        approx_eq(signed_lat, lat, 1e-6),
        "[{name}] lat mismatch: wrote {lat}, read {signed_lat}"
    );
    assert!(
        approx_eq(signed_lng, lng, 1e-6),
        "[{name}] lng mismatch: wrote {lng}, read {signed_lng}"
    );
    assert_eq!(read_lat_ref, expected_lat_ref, "[{name}] lat ref mismatch");
    assert_eq!(read_lng_ref, expected_lng_ref, "[{name}] lng ref mismatch");
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_tokyo() {
    round_trip_at("tokyo", 35.6586, 139.7454, 'N', 'E');
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_sydney() {
    round_trip_at("sydney", -33.8688, 151.2093, 'S', 'E');
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_buenos_aires() {
    round_trip_at("buenos_aires", -34.6037, -58.3816, 'S', 'W');
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_new_york() {
    round_trip_at("new_york", 40.7128, -74.0060, 'N', 'W');
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_with_altitude_above_sea_level() {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();
    pfp_exif::write_gps(&work, 35.6586, 139.7454, Some(150.0), None).expect("write_gps");
    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert!(summary.has_gps, "expected has_gps=true after write");
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn round_trip_with_altitude_below_sea_level() {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();
    pfp_exif::write_gps(&work, 35.6586, 139.7454, Some(-25.5), None).expect("write_gps");
    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert!(summary.has_gps);
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn datetime_original_written_or_created() {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();
    pfp_exif::write_gps(&work, 0.0, 0.0, None, Some("2024:06:15 12:00:00")).expect("write_gps");

    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert_eq!(summary.capture_time.as_deref(), Some("2024:06:15 12:00:00"));
}
