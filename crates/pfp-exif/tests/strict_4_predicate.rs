//! Phase 2 strict-4 GPS validity predicate (D-21).
//!
//! Mirrors `tests/round_trip.rs`: copy a real scanner fixture into a
//! tempdir, write the test coordinates via `pfp_exif::write_gps`, then
//! read back with `pfp_exif::read_summary` (which now uses
//! `is_gps_valid` internally) and `pfp_exif::gps_signed_decimal` and
//! assert the predicate matches expectation.

mod common;
use little_exif::{exif_tag::ExifTag, metadata::Metadata, rational::uR64};

fn predicate_at(name: &str, lat: f64, lng: f64, expect_valid: bool) {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();

    pfp_exif::write_gps(&work, lat, lng, None, None).expect("write_gps");
    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert_eq!(
        summary.has_gps, expect_valid,
        "[{name}] expected has_gps={expect_valid} for ({lat}, {lng})"
    );
    if expect_valid {
        let detail = pfp_exif::read_detail(&work).expect("read_detail");
        let (rlat, rlng) = detail.gps.expect("gps Some when valid");
        assert!(
            (rlat - lat).abs() < 1e-4,
            "[{name}] lat round-trip drift: {rlat} vs {lat}"
        );
        assert!(
            (rlng - lng).abs() < 1e-4,
            "[{name}] lng round-trip drift: {rlng} vs {lng}"
        );
    }
}

// Four valid corners
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn tokyo_n_e_valid() {
    predicate_at("tokyo", 35.6586, 139.7454, true);
}
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn sydney_s_e_valid() {
    predicate_at("sydney", -33.8688, 151.2093, true);
}
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn buenos_aires_s_w_valid() {
    predicate_at("buenos_aires", -34.6037, -58.3816, true);
}
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn new_york_n_w_valid() {
    predicate_at("new_york", 40.7128, -74.0060, true);
}

// Null Island: rejected as malformed (D-21).
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn null_island_invalid() {
    predicate_at("null_island", 0.0, 0.0, false);
}

// Predicate returns false on a JPEG with NO GPS tags (the scanner fixtures themselves).
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn no_gps_returns_false() {
    let src = common::scanner_fixtures().remove(0);
    // Don't write anything -- read the original.
    let summary = pfp_exif::read_summary(&src).expect("read_summary");
    assert!(!summary.has_gps, "scanner fixture must have no GPS by D-10");
}

// Synthesized in-memory tests for predicate edge cases that are hard to round-trip:
// missing GPSLatitudeRef + zero-denominator + malformed ref string.
#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn missing_lat_ref_returns_false() {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();

    let mut metadata = Metadata::new_from_path(&work).expect("read");
    // Set lat magnitude + lng pair WITHOUT setting GPSLatitudeRef.
    metadata.set_tag(ExifTag::GPSLatitude(vec![
        uR64 {
            nominator: 35,
            denominator: 1,
        },
        uR64 {
            nominator: 39,
            denominator: 1,
        },
        uR64 {
            nominator: 0,
            denominator: 1,
        },
    ]));
    metadata.set_tag(ExifTag::GPSLongitude(vec![
        uR64 {
            nominator: 139,
            denominator: 1,
        },
        uR64 {
            nominator: 44,
            denominator: 1,
        },
        uR64 {
            nominator: 0,
            denominator: 1,
        },
    ]));
    metadata.set_tag(ExifTag::GPSLongitudeRef("E".to_string()));
    // GPSLatitudeRef intentionally NOT set.
    metadata.write_to_file(&work).expect("write");

    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert!(!summary.has_gps, "missing GPSLatitudeRef must return false");
}

#[test]
#[ignore = "requires local scanner fixtures; run make test-gate"]
fn zero_denominator_returns_false() {
    let src = common::scanner_fixtures().remove(0);
    let tmp = tempfile::tempdir().unwrap();
    let work = tmp.path().join("photo.jpg");
    std::fs::copy(&src, &work).unwrap();

    let mut metadata = Metadata::new_from_path(&work).expect("read");
    metadata.set_tag(ExifTag::GPSLatitude(vec![
        uR64 {
            nominator: 35,
            denominator: 0,
        }, // <-- zero denom in degrees
        uR64 {
            nominator: 39,
            denominator: 1,
        },
        uR64 {
            nominator: 0,
            denominator: 1,
        },
    ]));
    metadata.set_tag(ExifTag::GPSLongitude(vec![
        uR64 {
            nominator: 139,
            denominator: 1,
        },
        uR64 {
            nominator: 44,
            denominator: 1,
        },
        uR64 {
            nominator: 0,
            denominator: 1,
        },
    ]));
    metadata.set_tag(ExifTag::GPSLatitudeRef("N".to_string()));
    metadata.set_tag(ExifTag::GPSLongitudeRef("E".to_string()));
    metadata.write_to_file(&work).expect("write");

    let summary = pfp_exif::read_summary(&work).expect("read_summary");
    assert!(!summary.has_gps, "zero-denominator must return false");
}
