//! Detailed photo metadata read path. Phase 2 wires this via the
//! `read_photo_meta` IPC command; the detail pane consumes the result.
//!
//! Reuses `crate::summary::gps_signed_decimal` for the strict-4 GPS pair
//! (D-22) and `crate::time::validate_dto_format` for D-24.

use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata, rational::uR64};

use crate::error::ExifError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PhotoDetail {
    /// Strict-4 valid (lat, lng); None for any invalid / absent state.
    pub gps: Option<(f64, f64)>,
    /// Signed altitude in meters. Sign comes from GPSAltitudeRef (0 = above
    /// sea level, 1 = below). None when GPSAltitude tag absent OR
    /// denominator is zero.
    pub altitude_m: Option<f64>,
    /// Validated DateTimeOriginal (D-24); None when absent OR malformed.
    pub capture_time: Option<String>,
}

pub fn read_detail(target: &Path) -> Result<PhotoDetail, ExifError> {
    let metadata = Metadata::new_from_path(target).map_err(|e| {
        ExifError::LittleExif(format!("read {}: {}", target.display(), e))
    })?;

    let gps = crate::summary::gps_signed_decimal(&metadata);
    let altitude_m = read_altitude(&metadata);
    let capture_time = metadata
        .get_tag(&ExifTag::DateTimeOriginal(String::new()))
        .next()
        .and_then(|tag| match tag {
            ExifTag::DateTimeOriginal(s) => Some(s.trim_end_matches('\0').to_string()),
            _ => None,
        })
        .filter(|s| crate::time::validate_dto_format(s).is_ok());

    Ok(PhotoDetail {
        gps,
        altitude_m,
        capture_time,
    })
}

fn read_altitude(metadata: &Metadata) -> Option<f64> {
    let alt_tag = metadata.get_tag(&ExifTag::GPSAltitude(Vec::new())).next()?;
    let alt_vec: &Vec<uR64> = match alt_tag {
        ExifTag::GPSAltitude(v) => v,
        _ => return None,
    };
    let r = alt_vec.first()?;
    if r.denominator == 0 {
        return None;
    }
    let magnitude = r.nominator as f64 / r.denominator as f64;

    // Sign from GPSAltitudeRef (Vec<u8>, single byte; 0 = above sea, 1 = below).
    let sign = metadata
        .get_tag(&ExifTag::GPSAltitudeRef(Vec::new()))
        .next()
        .and_then(|tag| match tag {
            ExifTag::GPSAltitudeRef(bytes) => bytes.first().copied(),
            _ => None,
        })
        .map(|b| if b == 1 { -1.0 } else { 1.0 })
        .unwrap_or(1.0);

    Some(sign * magnitude)
}
