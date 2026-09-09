//! ExifSummary read path. Plan 04 wires this via the `read_exif_summary` IPC command;
//! Phase 2 reuses it for the folder-list "has GPS / missing GPS" badges.
//!
//! Phase 2 (D-21, D-24): `has_gps` uses the strict-4 validity predicate
//! (`is_gps_valid`), and `capture_time` returns Some only when the value
//! passes `crate::time::validate_dto_format`.

use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata, rational::uR64};

use crate::error::ExifError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExifSummary {
    pub has_gps: bool,
    pub capture_time: Option<String>,
}

/// Read a quick summary: does the JPEG pass the strict-4 GPS validity
/// predicate (D-21), and what's its validated `DateTimeOriginal` (D-24)?
pub fn read_summary(target: &Path) -> Result<ExifSummary, ExifError> {
    let metadata = crate::jpeg_metadata::read(target)
        .map_err(|e| ExifError::LittleExif(format!("read {}: {}", target.display(), e)))?;

    let has_gps = is_gps_valid(&metadata);
    let capture_time = read_validated_dto(&metadata);

    Ok(ExifSummary {
        has_gps,
        capture_time,
    })
}

/// Backward-compatible name for the summary reader, which accepts EXIF-less JPEGs.
pub fn read_summary_or_default(target: &Path) -> Result<ExifSummary, ExifError> {
    read_summary(target)
}

/// Strict-4 validity predicate for the "has GPS" badge (D-21, FOLDER-03).
///
/// Returns `Some((lat_signed, lng_signed))` when ALL of:
/// - GPSLatitude, GPSLongitude, GPSLatitudeRef, GPSLongitudeRef tags present
/// - All denominators in the lat / lng RATIONAL64U triples are non-zero
/// - LatRef is exactly "N" or "S"; LngRef is exactly "E" or "W"
/// - Computed signed lat in (-90.0, 90.0) (exclusive); lng in (-180.0, 180.0)
/// - The pair (0.0, 0.0) is rejected as Null Island
///
/// Returns `None` for ANY failure mode. The boolean form is `is_gps_valid =
/// gps_signed_decimal(...).is_some()`.
pub fn gps_signed_decimal(metadata: &Metadata) -> Option<(f64, f64)> {
    let lat_dms = first_rational_triple(metadata, &ExifTag::GPSLatitude(Vec::new()))?;
    let lng_dms = first_rational_triple(metadata, &ExifTag::GPSLongitude(Vec::new()))?;
    let lat_ref = first_string(metadata, &ExifTag::GPSLatitudeRef(String::new()))?;
    let lng_ref = first_string(metadata, &ExifTag::GPSLongitudeRef(String::new()))?;

    let lat_sign = match lat_ref.as_str() {
        "N" => 1.0,
        "S" => -1.0,
        _ => return None,
    };
    let lng_sign = match lng_ref.as_str() {
        "E" => 1.0,
        "W" => -1.0,
        _ => return None,
    };

    let lat = lat_sign * dms_to_decimal(lat_dms)?;
    let lng = lng_sign * dms_to_decimal(lng_dms)?;

    if !(-90.0 < lat && lat < 90.0) {
        return None;
    }
    if !(-180.0 < lng && lng < 180.0) {
        return None;
    }
    if lat == 0.0 && lng == 0.0 {
        return None;
    }

    Some((lat, lng))
}

/// Boolean form of [`gps_signed_decimal`]; the predicate the
/// "has GPS / missing GPS" badge consumes.
pub fn is_gps_valid(metadata: &Metadata) -> bool {
    gps_signed_decimal(metadata).is_some()
}

fn dms_to_decimal(triple: [&uR64; 3]) -> Option<f64> {
    let to_f64 = |r: &uR64| -> Option<f64> {
        if r.denominator == 0 {
            return None;
        }
        Some(r.nominator as f64 / r.denominator as f64)
    };
    let d = to_f64(triple[0])?;
    let m = to_f64(triple[1])?;
    let s = to_f64(triple[2])?;
    Some(d + m / 60.0 + s / 3600.0)
}

fn first_rational_triple<'a>(metadata: &'a Metadata, sentinel: &ExifTag) -> Option<[&'a uR64; 3]> {
    let tag = metadata.get_tag(sentinel).next()?;
    let v: &Vec<uR64> = match tag {
        ExifTag::GPSLatitude(v) | ExifTag::GPSLongitude(v) => v,
        _ => return None,
    };
    if v.len() < 3 {
        return None;
    }
    Some([&v[0], &v[1], &v[2]])
}

fn first_string(metadata: &Metadata, sentinel: &ExifTag) -> Option<String> {
    let tag = metadata.get_tag(sentinel).next()?;
    let s = match tag {
        ExifTag::GPSLatitudeRef(s) | ExifTag::GPSLongitudeRef(s) => s,
        _ => return None,
    };
    Some(s.trim_end_matches('\0').to_string())
}

/// Read DateTimeOriginal, returning Some(s) ONLY when s passes
/// `crate::time::validate_dto_format` (D-24). Absent OR malformed both
/// collapse to None.
fn read_validated_dto(metadata: &Metadata) -> Option<String> {
    let s = metadata
        .get_tag(&ExifTag::DateTimeOriginal(String::new()))
        .next()
        .and_then(|tag| match tag {
            ExifTag::DateTimeOriginal(s) => Some(s.trim_end_matches('\0').to_string()),
            _ => None,
        })?;
    if crate::time::validate_dto_format(&s).is_ok() {
        Some(s)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use little_exif::exif_tag::ExifTag;
    use little_exif::metadata::Metadata;

    #[test]
    fn read_validated_dto_accepts_valid() {
        let mut metadata = Metadata::new();
        metadata.set_tag(ExifTag::DateTimeOriginal("2026:05:03 09:10:33".to_string()));
        assert_eq!(
            read_validated_dto(&metadata).as_deref(),
            Some("2026:05:03 09:10:33")
        );
    }

    #[test]
    fn read_validated_dto_rejects_malformed() {
        let mut metadata = Metadata::new();
        metadata.set_tag(ExifTag::DateTimeOriginal("not a date".to_string()));
        assert!(read_validated_dto(&metadata).is_none());
    }

    #[test]
    fn read_validated_dto_returns_none_when_absent() {
        let metadata = Metadata::new();
        assert!(read_validated_dto(&metadata).is_none());
    }
}
