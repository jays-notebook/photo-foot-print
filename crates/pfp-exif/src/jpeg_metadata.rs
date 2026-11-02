//! Locate EXIF by its APP1 signature, not by the first APP1 segment.
//! XMP-only JPEGs have empty EXIF; malformed EXIF must remain an error.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use little_exif::{filetype::FileExtension, metadata::Metadata};

/// Reject APP1 overflow before little_exif narrows the segment length to u16.
/// The length includes its own two bytes and the six-byte `Exif\0\0` header.
pub(crate) fn write(target: &Path, metadata: &Metadata) -> io::Result<()> {
    const MAX_TIFF_BYTES: usize = u16::MAX as usize - 2 - 6;
    let encoded_size = metadata.encode()?.len();
    if encoded_size > MAX_TIFF_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "EXIF metadata is too large for a JPEG APP1 segment ({encoded_size} bytes; maximum {MAX_TIFF_BYTES})"
            ),
        ));
    }
    metadata.write_to_file(target)?;
    // This is still the temporary file: a malformed encoding must never commit.
    read(target)?;
    Ok(())
}

pub(crate) fn read(target: &Path) -> io::Result<Metadata> {
    let mut reader = BufReader::new(File::open(target)?);
    let mut signature = [0; 2];
    reader.read_exact(&mut signature)?;
    if signature != [0xff, 0xd8] {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "Not a JPEG"));
    }
    loop {
        reader.read_exact(&mut signature)?;
        if signature[0] != 0xff {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid JPEG marker",
            ));
        }
        while signature[1] == 0xff {
            reader.read_exact(&mut signature[1..])?;
        }
        match signature[1] {
            0xda | 0xd9 => return Ok(Metadata::new()),
            0x01 | 0xd0..=0xd7 => continue,
            0x00 | 0xd8 => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Invalid JPEG marker",
                ))
            }
            _ => {}
        }
        let mut length_bytes = [0; 2];
        reader.read_exact(&mut length_bytes)?;
        let length = u16::from_be_bytes(length_bytes) as usize;
        if length < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Invalid JPEG segment length",
            ));
        }
        if signature[1] == 0xe1 {
            let mut payload = vec![0; length - 2];
            reader.read_exact(&mut payload)?;
            if payload.starts_with(b"Exif\0\0") {
                // Bound tag counts, offsets, nested IFDs and image data before
                // little_exif can allocate from untrusted TIFF declarations.
                crate::tiff_preflight::validate(&payload[6..])?;
                // TIFF offsets start after Exif\0\0, including thumbnails.
                let mut jpeg = vec![0xff, 0xd8, 0xff, 0xe1];
                jpeg.extend(length_bytes);
                jpeg.extend(payload);
                jpeg.extend([0xff, 0xd9]);
                return Metadata::new_from_vec(&jpeg, FileExtension::JPEG);
            }
        } else {
            reader.seek(SeekFrom::Current((length - 2) as i64))?;
        }
    }
}
