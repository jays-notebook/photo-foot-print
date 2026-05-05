//! Capture-time write (DateTimeOriginal + DateTimeDigitized companion) +
//! strict format validation.
//!
//! EXIF 2.31 spec format: `YYYY:MM:DD HH:MM:SS` (length 19, colons at byte
//! offsets 4, 7, 13, 16; space at offset 10; everything else ASCII digits).

use little_exif::{exif_tag::ExifTag, metadata::Metadata};

use crate::error::ExifError;

/// Set DateTimeOriginal AND DateTimeDigitized to the same 19-char value
/// (Phase 4 D-57 / Pitfall 13: writing only DateTimeOriginal leaves the
/// "digitized" timestamp stale, which Adobe Bridge keys off). Both tags
/// are mutated on the same `Metadata` instance before the surgical APP1
/// rewrite in `write_gps`, so the atomic-write contract persists either
/// both tags or neither.
///
/// Note on the tag identifier: in the EXIF 2.31 spec the 0x9004 tag is named
/// `DateTimeDigitized`. `little_exif 0.6.23` exposes that same TIFF tag under
/// the variant alias `ExifTag::CreateDate` (same id, same `STRING` format,
/// same 20-byte length). exiftool also recognizes both names for the same
/// underlying tag. The single `metadata.set_tag(ExifTag::CreateDate(...))`
/// call below writes EXIF 0x9004, which is what Pitfall 13 / D-57 require.
///
/// Visibility stays `pub(crate)` -- the only call site is `crate::write_gps`.
/// SubSec / OffsetTime tags are NOT touched in v1 (D-57: "preserve
/// whatever was on the file"). Renamed in Phase 4 from a single-tag helper
/// to reflect the widened scope (capture time = original + digitized).
pub(crate) fn set_capture_time(metadata: &mut Metadata, dto: &str) -> Result<(), ExifError> {
    validate_dto_format(dto)?;
    metadata.set_tag(ExifTag::DateTimeOriginal(dto.to_string()));
    // EXIF 0x9004 -- spec name: DateTimeDigitized. little_exif's variant alias
    // for the same tag id is `CreateDate`.
    metadata.set_tag(ExifTag::CreateDate(dto.to_string()));
    Ok(())
}

pub(crate) fn validate_dto_format(s: &str) -> Result<(), ExifError> {
    if s.len() != 19 {
        return Err(ExifError::InvalidDateTime(s.to_string()));
    }
    let ok = s.bytes().enumerate().all(|(i, b)| match i {
        4 | 7 => b == b':',
        10 => b == b' ',
        13 | 16 => b == b':',
        _ => b.is_ascii_digit(),
    });
    if !ok {
        return Err(ExifError::InvalidDateTime(s.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_format() {
        assert!(validate_dto_format("2024:01:15 14:30:00").is_ok());
        assert!(validate_dto_format("1970:01:01 00:00:00").is_ok());
        assert!(validate_dto_format("9999:12:31 23:59:59").is_ok());
    }

    #[test]
    fn rejects_iso_format() {
        assert!(validate_dto_format("2024-01-15T14:30:00").is_err());
    }

    #[test]
    fn rejects_short_format() {
        assert!(validate_dto_format("2024:01:15 14:30").is_err());
    }

    #[test]
    fn rejects_non_digit() {
        assert!(validate_dto_format("2024:01:15 14:30:0a").is_err());
    }

    #[test]
    fn rejects_wrong_separators() {
        assert!(validate_dto_format("2024.01.15 14:30:00").is_err());
        assert!(validate_dto_format("2024:01:15\t14:30:00").is_err());
    }

    /// Phase 4 D-57 / Pitfall 13: `set_capture_time` writes BOTH
    /// DateTimeOriginal (0x9003) AND DateTimeDigitized (0x9004 -- exposed by
    /// `little_exif 0.6.23` under the alias name `CreateDate`; same TIFF tag
    /// id, same wire format, same EXIF semantics). Catches a regression where
    /// the renamed helper drops the companion-write.
    #[test]
    fn set_capture_time_writes_both_dto_and_dt_digitized() {
        let mut metadata = little_exif::metadata::Metadata::new();
        super::set_capture_time(&mut metadata, "2024:01:15 14:30:00")
            .expect("valid format");

        let expected = "2024:01:15 14:30:00".to_string();
        let mut found_original = false;
        let mut found_digitized = false;
        for tag in &metadata {
            if let little_exif::exif_tag::ExifTag::DateTimeOriginal(s) = tag {
                assert_eq!(s, &expected);
                found_original = true;
            }
            if let little_exif::exif_tag::ExifTag::CreateDate(s) = tag {
                assert_eq!(s, &expected);
                found_digitized = true;
            }
        }
        assert!(found_original, "DateTimeOriginal must be set");
        assert!(
            found_digitized,
            "DateTimeDigitized (CreateDate, EXIF tag 0x9004) must be set (Pitfall 13)"
        );
    }

    /// Phase 4 D-57: validator gate is intact post-rename. Invalid format
    /// must still return `ExifError::InvalidDateTime`.
    #[test]
    fn set_capture_time_rejects_invalid_format() {
        let mut metadata = little_exif::metadata::Metadata::new();
        let err = super::set_capture_time(&mut metadata, "2024-01-15T14:30:00")
            .expect_err("ISO format must be rejected");
        match err {
            crate::error::ExifError::InvalidDateTime(s) => {
                assert_eq!(s, "2024-01-15T14:30:00");
            }
            other => panic!("expected InvalidDateTime, got: {other:?}"),
        }
    }
}
