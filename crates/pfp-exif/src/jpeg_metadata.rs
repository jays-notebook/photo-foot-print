//! Locate EXIF by its APP1 signature, not by the first APP1 segment.
//! XMP-only JPEGs have empty EXIF; malformed EXIF must remain an error.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use little_exif::{filetype::FileExtension, metadata::Metadata};

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
                // TIFF offsets are relative to this payload, including thumbnails.
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
