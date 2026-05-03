//! JPEG validity gates: extension match (cheap) + magic-byte sniff (3 bytes).
//! Defense-in-depth filter so that a non-JPEG file with a `.jpg` extension
//! is silently skipped (D-14, RESEARCH §Question 8).

use std::io::Read;
use std::path::Path;

pub fn has_jpeg_extension(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_ascii_lowercase())
            .as_deref(),
        Some("jpg") | Some("jpeg")
    )
}

/// Read first 3 bytes; check for SOI marker FF D8 FF.
/// Returns false on any I/O error (file gone, permission denied, etc.) -- the
/// caller treats this identically to "wrong magic" and skips the file.
pub fn is_real_jpeg(path: &Path) -> bool {
    let mut buf = [0u8; 3];
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };
    if f.read_exact(&mut buf).is_err() {
        return false;
    }
    buf == [0xFF, 0xD8, 0xFF]
}
