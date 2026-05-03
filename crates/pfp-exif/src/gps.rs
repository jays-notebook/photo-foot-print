//! Sign-aware decimal-degree -> DMS rational conversion + hemisphere ref derivation.
//!
//! PITFALLS.md §4 / §11 driving rules:
//! - EXIF stores GPS magnitudes UNSIGNED; sign lives in the Ref tag (N/S, E/W).
//! - Cameras encode rationals as (deg/1, min/1, sec*10000/10000). We match.
//! - Precision capped at ~6 decimal degrees (~11 cm at the equator).

use little_exif::rational::uR64;

/// Encoded magnitude as (deg, min, sec) of unsigned rationals. The triple is what
/// `little_exif::ExifTag::GPSLatitude` / `::GPSLongitude` accepts as `Vec<uR64>`.
#[derive(Debug, Clone)]
pub struct DmsRational {
    pub deg: uR64,
    pub min: uR64,
    pub sec: uR64,
}

/// Convert a signed decimal-degree value to (DMS rational triple, hemisphere ref char).
///
/// `positive_ref` and `negative_ref` are the EXIF Ref strings for the axis:
///   latitude  -> ('N', 'S')
///   longitude -> ('E', 'W')
pub fn signed_deg_to_rational_with_ref(
    value: f64,
    positive_ref: char,
    negative_ref: char,
) -> (DmsRational, char) {
    let r = if value >= 0.0 {
        positive_ref
    } else {
        negative_ref
    };
    let abs = value.abs();
    let deg_whole = abs.trunc() as u32;
    let min_full = (abs - deg_whole as f64) * 60.0;
    let min_whole = min_full.trunc() as u32;
    let sec_full = (min_full - min_whole as f64) * 60.0;
    // Cap precision at 1e-4 of an arc-second; matches camera EXIF style and
    // PITFALLS.md §11 "GPS Precision Pseudo-Honesty".
    let sec_scaled = (sec_full * 10_000.0).round() as u32;

    let dms = DmsRational {
        deg: uR64 {
            nominator: deg_whole,
            denominator: 1,
        },
        min: uR64 {
            nominator: min_whole,
            denominator: 1,
        },
        sec: uR64 {
            nominator: sec_scaled,
            denominator: 10_000,
        },
    };
    (dms, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    fn dms_to_decimal(dms: &DmsRational) -> f64 {
        let d = dms.deg.nominator as f64 / dms.deg.denominator as f64;
        let m = dms.min.nominator as f64 / dms.min.denominator as f64;
        let s = dms.sec.nominator as f64 / dms.sec.denominator as f64;
        d + m / 60.0 + s / 3600.0
    }

    #[test]
    fn tokyo_north_east() {
        let (dms_lat, lat_ref) = signed_deg_to_rational_with_ref(35.6586, 'N', 'S');
        let (dms_lng, lng_ref) = signed_deg_to_rational_with_ref(139.7454, 'E', 'W');
        assert_eq!(lat_ref, 'N');
        assert_eq!(lng_ref, 'E');
        assert!(approx_eq(dms_to_decimal(&dms_lat), 35.6586, 1e-6));
        assert!(approx_eq(dms_to_decimal(&dms_lng), 139.7454, 1e-6));
    }

    #[test]
    fn sydney_south_east() {
        let (dms_lat, lat_ref) = signed_deg_to_rational_with_ref(-33.8688, 'N', 'S');
        let (dms_lng, lng_ref) = signed_deg_to_rational_with_ref(151.2093, 'E', 'W');
        assert_eq!(lat_ref, 'S');
        assert_eq!(lng_ref, 'E');
        assert!(approx_eq(dms_to_decimal(&dms_lat), 33.8688, 1e-6));
        assert!(approx_eq(dms_to_decimal(&dms_lng), 151.2093, 1e-6));
    }

    #[test]
    fn buenos_aires_south_west() {
        let (_, lat_ref) = signed_deg_to_rational_with_ref(-34.6037, 'N', 'S');
        let (_, lng_ref) = signed_deg_to_rational_with_ref(-58.3816, 'E', 'W');
        assert_eq!(lat_ref, 'S');
        assert_eq!(lng_ref, 'W');
    }

    #[test]
    fn new_york_north_west() {
        let (_, lat_ref) = signed_deg_to_rational_with_ref(40.7128, 'N', 'S');
        let (_, lng_ref) = signed_deg_to_rational_with_ref(-74.0060, 'E', 'W');
        assert_eq!(lat_ref, 'N');
        assert_eq!(lng_ref, 'W');
    }

    #[test]
    fn zero_is_treated_as_positive_hemisphere() {
        let (_, lat_ref) = signed_deg_to_rational_with_ref(0.0, 'N', 'S');
        let (_, lng_ref) = signed_deg_to_rational_with_ref(0.0, 'E', 'W');
        assert_eq!(lat_ref, 'N');
        assert_eq!(lng_ref, 'E');
    }
}
