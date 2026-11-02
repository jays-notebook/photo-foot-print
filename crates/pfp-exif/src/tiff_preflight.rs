//! Bound every TIFF allocation source before little_exif 0.6.23 decodes EXIF.
//!
//! Keep traversal in sync with upstream `ifd::decode_ifd`, `ExifTag::from_u16`,
//! and `decode_tag_with_format_exceptions` when upgrading little_exif. In
//! particular, image-data tags are recognized in ALL groups, IFD pointers use
//! four raw bytes before format conversion, and SubIFD next links are ignored.
//! MakerNotes and unrecognized SubIFD tags remain opaque, as in that decoder.

use std::{collections::BTreeSet, io};

use little_exif::{
    exif_tag::{ExifTag, TagType},
    exif_tag_format::ExifTagFormat,
    ifd::ExifTagGroup,
};

const MAX_TIFF_BYTES: usize = u16::MAX as usize - 2 - 6;
const MAX_IFDS: usize = 64;
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 4096;
// A conservative weighted decoding budget, not an exact allocator measurement.
// Charging every occurrence also bounds aliases into the same source bytes.
// Charge 32 times raw data for conversion widening, simultaneous temporary
// copies and Vec capacity; per-entry/per-strip charges bound collection work.
// This is a decoding-work limit, not an exact process-memory/RSS ceiling.
const DECODE_BUDGET: usize = 4 * 1024 * 1024;

fn invalid(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

pub(crate) fn validate(tiff: &[u8]) -> io::Result<()> {
    if tiff.len() < 8 || tiff.len() > MAX_TIFF_BYTES {
        return Err(invalid("Invalid EXIF TIFF size"));
    }
    let little_endian = match &tiff[..2] {
        b"II" => true,
        b"MM" => false,
        _ => return Err(invalid("Invalid EXIF byte order")),
    };
    let mut validator = Validator {
        tiff,
        little_endian,
        visited: BTreeSet::new(),
        entries_left: MAX_ENTRIES,
        budget_left: DECODE_BUDGET,
    };
    if validator.u16(&tiff[2..4]) != 42 {
        return Err(invalid("Invalid EXIF TIFF magic"));
    }
    let mut offset = validator.u32(&tiff[4..8]);
    loop {
        offset = validator.ifd(offset as usize, ExifTagGroup::GENERIC, 0)?;
        if offset == 0 {
            return Ok(());
        }
    }
}

struct Validator<'a> {
    tiff: &'a [u8],
    little_endian: bool,
    visited: BTreeSet<usize>,
    entries_left: usize,
    budget_left: usize,
}

#[derive(Clone, Copy)]
struct UnsignedValues<'a> {
    bytes: &'a [u8],
    width: usize,
}

impl<'a> Validator<'a> {
    fn range(&self, offset: usize, length: usize) -> io::Result<&'a [u8]> {
        let end = offset
            .checked_add(length)
            .ok_or_else(|| invalid("EXIF offset overflow"))?;
        self.tiff
            .get(offset..end)
            .ok_or_else(|| invalid("EXIF data extends outside its APP1 segment"))
    }

    // Callers provide exactly sized, already bounds-checked slices.
    fn u16(&self, bytes: &[u8]) -> u16 {
        let bytes = [bytes[0], bytes[1]];
        if self.little_endian {
            u16::from_le_bytes(bytes)
        } else {
            u16::from_be_bytes(bytes)
        }
    }

    fn u32(&self, bytes: &[u8]) -> u32 {
        let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
        if self.little_endian {
            u32::from_le_bytes(bytes)
        } else {
            u32::from_be_bytes(bytes)
        }
    }

    fn charge(&mut self, bytes: usize) -> io::Result<()> {
        self.budget_left = self
            .budget_left
            .checked_sub(bytes)
            .ok_or_else(|| invalid("EXIF decoding budget exceeded"))?;
        Ok(())
    }

    fn ifd(&mut self, offset: usize, group: ExifTagGroup, depth: usize) -> io::Result<u32> {
        if offset < 8 || depth >= MAX_DEPTH || self.visited.len() >= MAX_IFDS {
            return Err(invalid("Invalid or excessive EXIF directory traversal"));
        }
        // Repeated directories are rejected, including cycles and shared IFDs.
        if !self.visited.insert(offset) {
            return Err(invalid("Repeated EXIF directory offset"));
        }
        let count = self.u16(self.range(offset, 2)?) as usize;
        self.entries_left = self
            .entries_left
            .checked_sub(count)
            .ok_or_else(|| invalid("EXIF entry budget exceeded"))?;
        // count is u16 and offset was checked above: these sizes cannot overflow.
        let directory = self.range(offset + 2, count * 12 + 4)?;
        let mut strips = (None, None);
        let mut thumbnail = (None, None);
        for entry in directory[..count * 12].as_chunks::<12>().0 {
            let tag_id = self.u16(&entry[..2]);
            let format = ExifTagFormat::from_u16(self.u16(&entry[2..4]))
                .ok_or_else(|| invalid("Unsupported EXIF field type"))?;
            let components = self.u32(&entry[4..8]);
            let byte_count = format
                .bytes_per_component()
                .checked_mul(components)
                .ok_or_else(|| invalid("EXIF component byte count overflow"))?
                as usize;
            let raw = if byte_count > 4 {
                self.range(self.u32(&entry[8..12]) as usize, byte_count)?
            } else {
                &entry[8..8 + byte_count]
            };
            // raw.len() is at most MAX_TIFF_BYTES, so this cannot overflow.
            self.charge(raw.len() * 32 + 512)?;
            let Ok(tag) = ExifTag::from_u16(tag_id, &group) else {
                continue;
            };
            if let TagType::IFD_OFFSET(child_group) = tag.get_tag_type() {
                if raw.len() != 4 {
                    return Err(invalid("Invalid EXIF directory pointer size"));
                }
                self.ifd(self.u32(raw) as usize, child_group, depth + 1)?;
                continue;
            }
            // These upstream conversion exceptions index/assert instead of
            // returning an error for malformed values. Reject them first.
            if tag.format() == ExifTagFormat::INT8U
                && format == ExifTagFormat::INT16U
                && raw
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .any(|value| self.u16(value) > 255)
            {
                return Err(invalid("EXIF byte conversion would lose data"));
            }
            if matches!(tag, ExifTag::GPSAltitudeRef(_))
                && format == ExifTagFormat::STRING
                && raw.is_empty()
            {
                return Err(invalid("Empty EXIF GPSAltitudeRef string"));
            }
            match tag {
                ExifTag::StripOffsets(..) => strips.0 = Some(Self::unsigned(raw, format)?),
                ExifTag::StripByteCounts(..) => strips.1 = Some(Self::unsigned(raw, format)?),
                ExifTag::ThumbnailOffset(..) => thumbnail.0 = Some(Self::unsigned(raw, format)?),
                ExifTag::ThumbnailLength(..) => thumbnail.1 = Some(Self::unsigned(raw, format)?),
                _ => {}
            }
        }
        // Like upstream, use the last occurrence of each image-data tag and zip
        // strip arrays. Borrow their bytes instead of allocating integer arrays.
        if let (Some(offsets), Some(lengths)) = strips {
            for (offset, length) in offsets
                .bytes
                .chunks_exact(offsets.width)
                .zip(lengths.bytes.chunks_exact(lengths.width))
            {
                self.image_data(self.unsigned_value(offset), self.unsigned_value(length))?;
            }
        }
        if let (Some(offsets), Some(lengths)) = thumbnail {
            if offsets.bytes.len() == offsets.width && lengths.bytes.len() == lengths.width {
                self.image_data(
                    self.unsigned_value(offsets.bytes),
                    self.unsigned_value(lengths.bytes),
                )?;
            }
        }
        Ok(self.u32(&directory[count * 12..]))
    }

    fn unsigned(bytes: &'a [u8], format: ExifTagFormat) -> io::Result<UnsignedValues<'a>> {
        // Upstream widens BYTE and SHORT data-offset values to LONG.
        let width = match format {
            ExifTagFormat::INT8U => 1,
            ExifTagFormat::INT16U => 2,
            ExifTagFormat::INT32U => 4,
            _ => return Err(invalid("Invalid EXIF image-data offset type")),
        };
        Ok(UnsignedValues { bytes, width })
    }

    fn unsigned_value(&self, bytes: &[u8]) -> u32 {
        match bytes.len() {
            1 => bytes[0] as u32,
            2 => self.u16(bytes) as u32,
            _ => self.u32(bytes),
        }
    }

    fn image_data(&mut self, offset: u32, length: u32) -> io::Result<()> {
        let data = self.range(offset as usize, length as usize)?;
        self.charge(data.len() * 2 + 64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Order {
        Little,
        Big,
    }

    impl Order {
        fn u16(self, value: u16) -> [u8; 2] {
            match self {
                Self::Little => value.to_le_bytes(),
                Self::Big => value.to_be_bytes(),
            }
        }

        fn u32(self, value: u32) -> [u8; 4] {
            match self {
                Self::Little => value.to_le_bytes(),
                Self::Big => value.to_be_bytes(),
            }
        }
    }

    // Each tuple is the on-disk tag, type, component count, and raw value/offset.
    fn tiff(order: Order, entries: &[(u16, u16, u32, [u8; 4])], next: u32) -> Vec<u8> {
        let mut bytes = match order {
            Order::Little => b"II".to_vec(),
            Order::Big => b"MM".to_vec(),
        };
        bytes.extend(order.u16(42));
        bytes.extend(order.u32(8));
        append_ifd(&mut bytes, order, entries, next);
        bytes
    }

    fn append_ifd(
        bytes: &mut Vec<u8>,
        order: Order,
        entries: &[(u16, u16, u32, [u8; 4])],
        next: u32,
    ) {
        bytes.extend(order.u16(entries.len() as u16));
        for &(tag, format, count, value) in entries {
            bytes.extend(order.u16(tag));
            bytes.extend(order.u16(format));
            bytes.extend(order.u32(count));
            bytes.extend(value);
        }
        bytes.extend(order.u32(next));
    }

    fn rejected(bytes: &[u8]) {
        assert_eq!(
            validate(bytes).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }

    #[test]
    fn rejects_tag_counts_and_offsets_outside_the_segment() {
        for order in [Order::Little, Order::Big] {
            for (format, count, offset) in [
                (7, u32::MAX, 26),
                (5, u32::MAX, 26),
                (7, 5, 25),
                (7, 5, u32::MAX),
            ] {
                rejected(&tiff(
                    order,
                    &[(0xc100, format, count, order.u32(offset))],
                    0,
                ));
            }
        }
    }

    #[test]
    fn rejects_invalid_headers_and_incomplete_directories() {
        for bytes in [
            Vec::new(),
            b"II\x2a\0\x08\0\0\0".to_vec(),
            b"XX\x2a\0\x08\0\0\0\0\0\0\0\0\0".to_vec(),
            b"II\x2b\0\x08\0\0\0\0\0\0\0\0\0".to_vec(),
            b"II\x2a\0\xff\xff\xff\xff".to_vec(),
            b"II\x2a\0\0\0\0\0".to_vec(),
            b"II\x2a\0\x08\0\0\0\xff\xff\0\0\0\0".to_vec(),
        ] {
            rejected(&bytes);
        }
        for format in [0, 13, u16::MAX] {
            rejected(&tiff(Order::Little, &[(0xc100, format, 1, [0; 4])], 0));
        }
    }

    #[test]
    fn accepts_inline_and_external_values_for_every_supported_type_and_byte_order() {
        for order in [Order::Little, Order::Big] {
            for (format, width) in [
                (1, 1),
                (2, 1),
                (3, 2),
                (4, 4),
                (5, 8),
                (6, 1),
                (7, 1),
                (8, 2),
                (9, 4),
                (10, 8),
                (11, 4),
                (12, 8),
            ] {
                for count in [0, 1, 5] {
                    let size = width * count;
                    let value = if size > 4 { order.u32(26) } else { [0; 4] };
                    let mut bytes = tiff(order, &[(0xc100, format, count, value)], 0);
                    if size > 4 {
                        bytes.resize(26 + size as usize, 0);
                    }
                    validate(&bytes).unwrap();
                }
            }
        }
    }

    #[test]
    fn follows_exif_gps_and_interop_ifds_using_raw_pointer_bytes() {
        for order in [Order::Little, Order::Big] {
            // Upstream follows any exactly four raw bytes before format conversion.
            for (format, count) in [(1, 4), (3, 2), (4, 1), (7, 4)] {
                for tag in [0x8769, 0x8825] {
                    let mut bytes = tiff(order, &[(tag, format, count, order.u32(26))], 0);
                    append_ifd(
                        &mut bytes,
                        order,
                        &[(0xc100, 7, u32::MAX, order.u32(44))],
                        0,
                    );
                    rejected(&bytes);
                }
            }
            let mut bytes = tiff(order, &[(0x8769, 4, 1, order.u32(26))], 0);
            append_ifd(&mut bytes, order, &[(0xa005, 4, 1, order.u32(44))], 0);
            append_ifd(
                &mut bytes,
                order,
                &[(0xc100, 7, u32::MAX, order.u32(62))],
                0,
            );
            rejected(&bytes);
        }
    }

    #[test]
    fn rejects_missing_and_repeated_directory_references() {
        let order = Order::Little;
        for (format, count, offset) in [(4, 1, 4096), (4, 1, 8), (3, 1, 26), (4, 2, 26)] {
            rejected(&tiff(
                order,
                &[(0x8769, format, count, order.u32(offset))],
                0,
            ));
        }
        rejected(&tiff(order, &[], 8));
        let mut bytes = tiff(order, &[], 14);
        append_ifd(&mut bytes, order, &[], 8);
        rejected(&bytes);
    }

    #[test]
    fn rejects_empty_gps_altitude_ref_string_before_upstream_indexes_it() {
        for order in [Order::Little, Order::Big] {
            let mut bytes = tiff(order, &[(0x8825, 4, 1, order.u32(26))], 0);
            append_ifd(&mut bytes, order, &[(0x0005, 2, 0, [0; 4])], 0);
            rejected(&bytes);
        }
    }

    #[test]
    fn rejects_lossy_byte_conversion_before_upstream_asserts() {
        for order in [Order::Little, Order::Big] {
            let value = order.u16(256);
            let mut bytes = tiff(order, &[(0x8825, 4, 1, order.u32(26))], 0);
            append_ifd(
                &mut bytes,
                order,
                &[(0x0000, 3, 1, [value[0], value[1], 0, 0])],
                0,
            );
            rejected(&bytes);
        }
    }

    #[test]
    fn bounds_thumbnail_and_strip_data_in_every_directory_group() {
        for order in [Order::Little, Order::Big] {
            for pair in [(0x0111, 0x0117), (0x0201, 0x0202)] {
                for group_path in [vec![], vec![0x8769], vec![0x8825], vec![0x8769, 0xa005]] {
                    for (format, count) in [(1, 1), (3, 1), (4, 1)] {
                        let mut bytes = tiff(order, &[], 0);
                        bytes.truncate(8);
                        for tag in &group_path {
                            let next = bytes.len() as u32 + 18;
                            append_ifd(&mut bytes, order, &[(*tag, 4, 1, order.u32(next))], 0);
                        }
                        // 255 is outside this tiny TIFF for every accepted integer type.
                        let raw = match format {
                            1 => [255, 0, 0, 0],
                            3 => {
                                let value = order.u16(255);
                                [value[0], value[1], 0, 0]
                            }
                            _ => order.u32(u32::MAX),
                        };
                        append_ifd(
                            &mut bytes,
                            order,
                            &[(pair.0, 4, 1, order.u32(8)), (pair.1, format, count, raw)],
                            0,
                        );
                        rejected(&bytes);
                    }
                }
            }
        }
    }

    #[test]
    fn rejects_amplification_from_overlapping_tag_values() {
        let order = Order::Little;
        let entries = vec![(0xc100, 7, 60_000, order.u32(206)); 16];
        let mut bytes = tiff(order, &entries, 0);
        bytes.resize(60_206, 0);
        rejected(&bytes);
    }

    #[test]
    fn rejects_amplification_from_repeated_strips() {
        let order = Order::Little;
        let mut bytes = tiff(
            order,
            &[
                (0x0111, 4, 256, order.u32(38)),
                (0x0117, 4, 256, order.u32(1062)),
            ],
            0,
        );
        for _ in 0..256 {
            bytes.extend(order.u32(2086));
        }
        for _ in 0..256 {
            bytes.extend(order.u32(40_000));
        }
        bytes.resize(42_086, 0);
        rejected(&bytes);
    }

    #[test]
    fn rejects_excessive_directory_and_entry_work() {
        let order = Order::Little;
        let mut bytes = tiff(order, &[], 14);
        for index in 1..128 {
            append_ifd(
                &mut bytes,
                order,
                &[],
                if index == 127 { 0 } else { 8 + (index + 1) * 6 },
            );
        }
        rejected(&bytes);
        rejected(&tiff(order, &vec![(0xc100, 1, 1, [0; 4]); 4500], 0));
    }

    #[test]
    fn accepts_nested_metadata_and_embedded_images_without_interpreting_opaque_values() {
        for order in [Order::Little, Order::Big] {
            let mut bytes = tiff(order, &[(0x8769, 4, 1, order.u32(26))], 0);
            append_ifd(
                &mut bytes,
                order,
                &[(0xa005, 4, 1, order.u32(56)), (0x927c, 7, 8, order.u32(62))],
                0,
            );
            append_ifd(&mut bytes, order, &[], u32::MAX); // SubIFD next links are ignored upstream.
            bytes.extend([255; 8]); // MakerNote remains opaque.
            validate(&bytes).unwrap();

            for pair in [(0x0111, 0x0117), (0x0201, 0x0202)] {
                let mut bytes = tiff(
                    order,
                    &[(pair.0, 4, 1, order.u32(38)), (pair.1, 4, 1, order.u32(4))],
                    0,
                );
                bytes.extend([0xff, 0xd8, 0xff, 0xd9]);
                validate(&bytes).unwrap();
            }
        }
    }
}
