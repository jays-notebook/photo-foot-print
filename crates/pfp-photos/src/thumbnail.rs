//! Thumbnail pipeline: decode JPEG via `image` (zune-jpeg backend),
//! downsample via `fast_image_resize` (SIMD Lanczos3), re-encode JPEG
//! at quality 85 (D-20). Pure CPU work; no I/O beyond File::open.
//!
//! Pitfall 2-J: malformed-JPEG decode failures propagate as
//! `PhotosError::Decode(...)` -- never panic.

use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor};
use std::path::Path;

use fast_image_resize::{
    images::Image, FilterType, PixelType, ResizeAlg, ResizeOptions, Resizer,
};
use image::{codecs::jpeg::JpegEncoder, ColorType, ImageEncoder, ImageReader};

use crate::error::PhotosError;

/// D-20: long edge 256 px (aspect-preserved), JPEG quality 85.
pub const TARGET_LONG_EDGE: u32 = 256;
pub const JPEG_QUALITY: u8 = 85;

pub fn decode_resize_encode(src: &Path) -> Result<Vec<u8>, PhotosError> {
    let file = File::open(src)?;
    let reader = ImageReader::new(BufReader::new(file))
        .with_guessed_format()
        .map_err(|e| PhotosError::Decode(format!("guess format: {e}")))?;
    let dynamic = reader
        .decode()
        .map_err(|e| PhotosError::Decode(e.to_string()))?;
    let rgb = dynamic.to_rgb8();
    let (src_w, src_h) = rgb.dimensions();

    if src_w == 0 || src_h == 0 {
        return Err(PhotosError::Decode("zero-dimension image".to_string()));
    }

    let (dst_w, dst_h) = if src_w >= src_h {
        let scale = TARGET_LONG_EDGE as f64 / src_w as f64;
        let h = (src_h as f64 * scale).round().max(1.0) as u32;
        (TARGET_LONG_EDGE, h)
    } else {
        let scale = TARGET_LONG_EDGE as f64 / src_h as f64;
        let w = (src_w as f64 * scale).round().max(1.0) as u32;
        (w, TARGET_LONG_EDGE)
    };

    let src_view = Image::from_vec_u8(src_w, src_h, rgb.into_raw(), PixelType::U8x3)
        .map_err(|e| PhotosError::Resize(e.to_string()))?;
    let mut dst = Image::new(dst_w, dst_h, PixelType::U8x3);
    let mut resizer = Resizer::new();
    let options =
        ResizeOptions::new().resize_alg(ResizeAlg::Convolution(FilterType::Lanczos3));
    resizer
        .resize(&src_view, &mut dst, &options)
        .map_err(|e| PhotosError::Resize(e.to_string()))?;

    let mut out = Vec::with_capacity(32 * 1024);
    {
        let mut writer = BufWriter::new(Cursor::new(&mut out));
        let encoder = JpegEncoder::new_with_quality(&mut writer, JPEG_QUALITY);
        encoder
            .write_image(dst.buffer(), dst_w, dst_h, ColorType::Rgb8.into())
            .map_err(|e| PhotosError::Encode(e.to_string()))?;
    }
    Ok(out)
}
