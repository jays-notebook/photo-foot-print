//! pfp-exif -- surgical EXIF GPS + DateTimeOriginal write for JPEGs.
//!
//! D-08 invariant: zero `tauri::*` symbols. Pure Rust on top of `little_exif`.
//! Used by:
//!   - `crates/src-tauri/src/commands/exif.rs` (Plan 04) -- thin command wrappers
//!   - `crates/pfp-photos`              (Phase 2) -- reuses `read_summary` for badges
//!
//! Architectural rules:
//!   - GPS magnitudes are UNSIGNED rationals; sign lives in the Ref tag (PITFALLS §4).
//!   - Precision capped at ~6 decimal degrees (PITFALLS §11).
//!   - Writes are surgical -- `little_exif::Metadata::set_tag` + `write_to_file`
//!     replace only the EXIF APP1 segment. MakerNotes survive byte-for-byte
//!     (Phase 1 success criterion #3).
//!   - All writes go through `crate::atomic::write_via_temp` (Plan 03 makes it durable).

pub mod atomic;
pub mod error;
mod gps;
pub mod summary;
mod time;

pub use crate::error::ExifError;
pub use crate::summary::{read_summary, ExifSummary};

use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata, rational::uR64};

use crate::gps::signed_deg_to_rational_with_ref;
use crate::time::set_datetime_original;

/// Write GPS coordinates (and optionally altitude + DateTimeOriginal) to a JPEG.
///
/// Per CONTEXT.md "Claude's Discretion §GPS write tag set in v1": altitude IS
/// written when `Some` (no UI surface in v1). EXIF-05 / EXIF-06 satisfied here.
pub fn write_gps(
    target: &Path,
    lat: f64,
    lng: f64,
    altitude_m: Option<f64>,
    dto: Option<&str>,
) -> Result<(), ExifError> {
    if let Some(s) = dto {
        // Validate up front -- fail before touching the file.
        crate::time::validate_dto_format(s)?;
    }

    crate::atomic::write_via_temp(target, |tmp_path| {
        // Load metadata from the SAME tmp_path the atomic helper just populated
        // (by copying the original).
        let mut metadata = Metadata::new_from_path(tmp_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("little_exif read: {e}"),
            )
        })?;

        let (lat_dms, lat_ref) = signed_deg_to_rational_with_ref(lat, 'N', 'S');
        let (lng_dms, lng_ref) = signed_deg_to_rational_with_ref(lng, 'E', 'W');

        metadata.set_tag(ExifTag::GPSLatitude(vec![
            lat_dms.deg.clone(),
            lat_dms.min.clone(),
            lat_dms.sec.clone(),
        ]));
        metadata.set_tag(ExifTag::GPSLatitudeRef(lat_ref.to_string()));
        metadata.set_tag(ExifTag::GPSLongitude(vec![
            lng_dms.deg.clone(),
            lng_dms.min.clone(),
            lng_dms.sec.clone(),
        ]));
        metadata.set_tag(ExifTag::GPSLongitudeRef(lng_ref.to_string()));

        if let Some(alt) = altitude_m {
            let abs_alt = alt.abs();
            metadata.set_tag(ExifTag::GPSAltitude(vec![uR64 {
                nominator: (abs_alt * 1_000.0).round() as u32,
                denominator: 1_000,
            }]));
            metadata.set_tag(ExifTag::GPSAltitudeRef(vec![if alt >= 0.0 { 0u8 } else { 1u8 }]));
        }

        if let Some(dto_str) = dto {
            set_datetime_original(&mut metadata, dto_str).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
            })?;
        }

        metadata.write_to_file(tmp_path).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("little_exif write: {e}"),
            )
        })?;

        Ok(())
    })?;

    Ok(())
}
