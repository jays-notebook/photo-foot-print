//! ExifSummary read path. Plan 04 wires this via the `read_exif_summary` IPC command;
//! Phase 2 reuses it for the folder-list "has GPS / missing GPS" badges.

use std::path::Path;

use little_exif::{exif_tag::ExifTag, metadata::Metadata};

use crate::error::ExifError;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExifSummary {
    pub has_gps: bool,
    pub capture_time: Option<String>,
}

/// Read a quick summary: does the JPEG have all four required GPS tags, and what's
/// its DateTimeOriginal?
///
/// Phase 1 definition of `has_gps`: the four required GPS tags (Lat, Lng, LatRef,
/// LngRef) are PRESENT. Phase 2 (REQUIREMENTS.md FOLDER-03) adds the strict-validity
/// predicate (no zero denominators, not Null Island). Phase 1 is presence-only.
pub fn read_summary(target: &Path) -> Result<ExifSummary, ExifError> {
    let metadata = Metadata::new_from_path(target).map_err(|e| {
        ExifError::LittleExif(format!("read {}: {}", target.display(), e))
    })?;

    // little_exif 0.6.23 `get_tag(&ExifTag)` returns an iterator of matching tags
    // in the IFD that owns the tag's group. We pass sentinel variants because the
    // function dispatches on tag hex + group, not on inner data.
    let has_lat = metadata
        .get_tag(&ExifTag::GPSLatitude(Vec::new()))
        .next()
        .is_some();
    let has_lng = metadata
        .get_tag(&ExifTag::GPSLongitude(Vec::new()))
        .next()
        .is_some();
    let has_lat_ref = metadata
        .get_tag(&ExifTag::GPSLatitudeRef(String::new()))
        .next()
        .is_some();
    let has_lng_ref = metadata
        .get_tag(&ExifTag::GPSLongitudeRef(String::new()))
        .next()
        .is_some();

    let has_gps = has_lat && has_lng && has_lat_ref && has_lng_ref;

    let capture_time = metadata
        .get_tag(&ExifTag::DateTimeOriginal(String::new()))
        .next()
        .and_then(|tag| match tag {
            ExifTag::DateTimeOriginal(s) => Some(s.clone()),
            _ => None,
        });

    Ok(ExifSummary {
        has_gps,
        capture_time,
    })
}
