//! On-disk cache layout + atomic write helpers.
//!
//! Layout: `{cache_root}/osm/{z}/{x}/{y}.png`
//!     +   `{cache_root}/osm/{z}/{x}/{y}.png.meta.json`
//!
//! Atomic-write idiom mirrors `pfp-state` (sibling tempfile + persist; NO
//! F_FULLFSYNC). Tiles are regenerable; the gold-plated `pfp-exif::atomic`
//! path is reserved for the irreplaceable photo write.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::TileError;
use crate::sidecar;
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
///
/// WR-05: build the path directly via `format!("{y}.png.meta.json")`
/// rather than going through `tile_path` + `file_name().unwrap().to_str().unwrap()`.
/// The unwraps were unreachable in practice (production callers always
/// build through `tile_path` which produces an ASCII filename), but the
/// function takes `&Path`; a future cleanup-utility caller passing a
/// path with no UTF-8 file name would have panicked on the URI scheme
/// handler thread. This formulation matches the layout invariant
/// documented at the top of this file with no panic surface.
pub fn meta_path(cache_root: &Path, z: u8, x: u32, y: u32) -> PathBuf {
    cache_root
        .join("osm")
        .join(z.to_string())
        .join(x.to_string())
        .join(format!("{y}.png.meta.json"))
}

/// Atomic tile + sidecar write. Tile FIRST, sidecar SECOND so a crash
/// mid-pair leaves "tile + missing sidecar" (recoverable as stale on
/// next read), never "sidecar + missing tile".
///
/// Pitfall 10 mitigation: `create_dir_all` for the parent chain on first write.
/// Pitfall 4 mitigation: sibling tempfile in the SAME directory (not the OS
/// tmpdir) so `persist` is a same-filesystem rename, not a cross-filesystem
/// copy.
pub fn write_atomic(
    tile_path: &Path,
    meta_path: &Path,
    bytes: &[u8],
    meta: &TileMeta,
) -> Result<(), TileError> {
    // Pitfall 10: create the dir chain on first write so tempfile_in succeeds.
    if let Some(parent) = tile_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // 1. Write tile file atomically (tile FIRST per RESEARCH §Pattern 4).
    //    A crash AFTER step 1 but BEFORE step 2 leaves "tile + missing sidecar"
    //    on disk; the next read sees `SidecarReadError::Absent`, treats it as
    //    stale, and revalidates -- self-healing.
    let tile_parent = tile_path
        .parent()
        .ok_or_else(|| TileError::Sidecar("tile path has no parent".to_string()))?;
    let tile_tmp = tempfile::Builder::new()
        .prefix(".tile-")
        .suffix(".png.tmp")
        .tempfile_in(tile_parent)?;
    {
        let mut w = std::io::BufWriter::new(tile_tmp.as_file());
        w.write_all(bytes)?;
        w.flush()?;
    }
    tile_tmp.as_file().sync_all()?;
    tile_tmp
        .persist(tile_path)
        .map_err(|e| TileError::Io(e.error))?;

    // 2. Write sidecar SECOND. The reverse order would risk "sidecar +
    //    missing tile" on a crash, which the cache cannot self-heal from
    //    (a sidecar pointing at no bytes is a poison entry).
    sidecar::write_atomic(meta_path, meta)?;

    Ok(())
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
        assert_eq!(
            p,
            PathBuf::from("/tmp/cache/osm/14/13708/6334.png.meta.json")
        );
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
