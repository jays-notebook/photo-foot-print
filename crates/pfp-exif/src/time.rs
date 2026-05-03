//! DateTimeOriginal write + strict format validation.
//!
//! EXIF 2.31 spec format: `YYYY:MM:DD HH:MM:SS` (length 19, colons at byte
//! offsets 4, 7, 13, 16; space at offset 10; everything else ASCII digits).

use little_exif::{exif_tag::ExifTag, metadata::Metadata};

use crate::error::ExifError;

/// Set DateTimeOriginal on the given metadata. Creates the tag if absent --
/// `little_exif::Metadata::set_tag` "automatically determines appropriate IFD placement".
pub(crate) fn set_datetime_original(metadata: &mut Metadata, dto: &str) -> Result<(), ExifError> {
    validate_dto_format(dto)?;
    metadata.set_tag(ExifTag::DateTimeOriginal(dto.to_string()));
    Ok(())
}

pub(crate) fn validate_dto_format(_s: &str) -> Result<(), ExifError> {
    // RED stub: GREEN commit replaces the body with the real validator. For now
    // accept everything so the test harness gets to run; the rejecting tests
    // will fail and drive the GREEN commit's logic.
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
}
