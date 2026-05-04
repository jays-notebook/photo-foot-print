//! On-disk cache layout + atomic write helpers.
//!
//! Layout: `{cache_root}/osm/{z}/{x}/{y}.png`
//!     +   `{cache_root}/osm/{z}/{x}/{y}.png.meta.json`
//!
//! Atomic-write idiom mirrors `pfp-state` (sibling tempfile + persist; NO
//! F_FULLFSYNC). Tiles are regenerable; the gold-plated `pfp-exif::atomic`
//! path is reserved for the irreplaceable photo write.

use std::path::{Path, PathBuf};

use crate::error::TileError;
use crate::sidecar::TileMeta;

/// Compute the absolute path for the tile PNG file.
/// Pure path arithmetic -- does not touch the filesystem.
pub fn tile_path(cache_root: &Path, z: u8, x: u32, y: u32) -> PathBuf {
    cache_root
        .join("osm")
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png"))
}

/// Compute the absolute path for the tile sidecar JSON.
pub fn meta_path(cache_root: &Path, z: u8, x: u32, y: u32) -> PathBuf {
    let mut p = tile_path(cache_root, z, x, y);
    // Append ".meta.json" by manipulating the file name; cannot use
    // `set_extension` because that would replace ".png".
    let new_name = format!(
        "{}.meta.json",
        p.file_name().unwrap().to_str().unwrap()
    );
    p.set_file_name(new_name);
    p
}

/// Atomic tile + sidecar write. Tile FIRST, sidecar SECOND so a crash
/// mid-pair leaves "tile + missing sidecar" (recoverable as stale on
/// next read), never "sidecar + missing tile".
/// Plan 02 implements the body.
pub fn write_atomic(
    _tile_path: &Path,
    _meta_path: &Path,
    _bytes: &[u8],
    _meta: &TileMeta,
) -> Result<(), TileError> {
    unimplemented!("Plan 02 implements cache::write_atomic")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn tile_path_layout_is_osm_z_x_y() {
        let root = PathBuf::from("/tmp/cache");
        let p = tile_path(&root, 14, 13708, 6334);
        assert_eq!(p, PathBuf::from("/tmp/cache/osm/14/13708/6334.png"));
    }

    #[test]
    fn meta_path_appends_dot_meta_dot_json() {
        let root = PathBuf::from("/tmp/cache");
        let p = meta_path(&root, 14, 13708, 6334);
        assert_eq!(p, PathBuf::from("/tmp/cache/osm/14/13708/6334.png.meta.json"));
    }

    #[test]
    fn write_atomic_creates_parent_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tp = tile_path(root, 14, 13708, 6334);
        let mp = meta_path(root, 14, 13708, 6334);
        let m = TileMeta {
            schema_version: 1,
            max_age_secs: 604_800,
            fetched_at_unix: 0,
            etag: None,
            last_modified: None,
        };
        write_atomic(&tp, &mp, b"PNGFAKE", &m).unwrap();
        assert!(tp.exists());
        assert!(mp.exists());
    }

    #[test]
    fn write_atomic_writes_tile_bytes_byte_identical() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let tp = tile_path(root, 5, 1, 1);
        let mp = meta_path(root, 5, 1, 1);
        let m = TileMeta {
            schema_version: 1,
            max_age_secs: 604_800,
            fetched_at_unix: 0,
            etag: None,
            last_modified: None,
        };
        let payload = b"\x89PNG\r\n\x1a\n_fake_tile_bytes";
        write_atomic(&tp, &mp, payload, &m).unwrap();
        let read_back = std::fs::read(&tp).unwrap();
        assert_eq!(read_back, payload);
        let read_meta = crate::sidecar::read(&mp).unwrap();
        assert_eq!(read_meta, m);
    }

    #[test]
    fn write_order_is_tile_first_then_sidecar() {
        // Source-level invariant check -- read this very file via include_str!
        // and assert the comment markers appear in tile-first order.
        let src = include_str!("cache.rs");
        let one = src.find("// 1.").expect("missing // 1. marker");
        let two = src.find("// 2.").expect("missing // 2. marker");
        assert!(one < two, "tile (// 1.) must come before sidecar (// 2.)");
    }
}
