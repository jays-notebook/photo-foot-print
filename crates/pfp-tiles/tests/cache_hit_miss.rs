//! Integration tests for `TileCache::get_tile` cache-hit / stale-revalidate
//! / sidecar-parse-error branches. These tests deliberately exercise paths
//! that never call upstream (cache hit, parse-error path serves cached bytes
//! synchronously) so they can run without a mock HTTP server.
//!
//! The cache-miss path (one synchronous upstream GET) is covered by
//! `tests/fetch_upstream.rs` against `fetch::fetch_upstream_url`.

use std::path::PathBuf;

use pfp_tiles::{build_osm_client, TileCache, TileResponse};

fn cache_with_root(root: PathBuf) -> TileCache {
    let http = build_osm_client().expect("build osm client");
    TileCache::new(root, http)
}

fn write_fixture_tile(
    root: &std::path::Path,
    z: u8,
    x: u32,
    y: u32,
    bytes: &[u8],
    meta: pfp_tiles::sidecar::TileMeta,
) {
    let tp = pfp_tiles::cache::tile_path(root, z, x, y);
    let mp = pfp_tiles::cache::meta_path(root, z, x, y);
    pfp_tiles::cache::write_atomic(&tp, &mp, bytes, &meta).unwrap();
}

#[tokio::test]
async fn cache_hit_returns_cached_bytes_without_upstream() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_root(dir.path().to_path_buf());

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let fresh_meta = pfp_tiles::sidecar::TileMeta {
        schema_version: 1,
        max_age_secs: 7 * 86_400,
        fetched_at_unix: now,
        etag: None,
        last_modified: None,
    };
    let payload = b"\x89PNG\r\n\x1a\n_cached";
    write_fixture_tile(dir.path(), 14, 13708, 6334, payload, fresh_meta);

    let TileResponse {
        bytes,
        from_cache,
        was_stale,
    } = cache.get_tile(14, 13708, 6334).await.unwrap();
    assert_eq!(bytes, payload);
    assert!(from_cache);
    assert!(!was_stale);
}

#[tokio::test]
async fn stale_cache_returns_cached_bytes_immediately() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_root(dir.path().to_path_buf());

    // fetched_at = epoch 0 -> very stale by 7-day floor.
    let stale_meta = pfp_tiles::sidecar::TileMeta {
        schema_version: 1,
        max_age_secs: 7 * 86_400,
        fetched_at_unix: 0,
        etag: None,
        last_modified: None,
    };
    let payload = b"_stale_bytes";
    write_fixture_tile(dir.path(), 5, 1, 1, payload, stale_meta);

    let TileResponse {
        bytes,
        from_cache,
        was_stale,
    } = cache.get_tile(5, 1, 1).await.unwrap();
    assert_eq!(bytes, payload);
    assert!(from_cache);
    assert!(was_stale);
    // Background revalidation will fail (real network) -- that's fine,
    // we asserted the synchronous path. Sleep briefly to let the spawned
    // task observe the failure (and not drop a runtime task mid-flight
    // at process exit).
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn corrupt_sidecar_treated_as_stale_returns_cached_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_root(dir.path().to_path_buf());

    let z = 7;
    let x = 22;
    let y = 33;
    let tp = pfp_tiles::cache::tile_path(dir.path(), z, x, y);
    let mp = pfp_tiles::cache::meta_path(dir.path(), z, x, y);
    std::fs::create_dir_all(tp.parent().unwrap()).unwrap();
    std::fs::write(&tp, b"_tile").unwrap();
    std::fs::write(&mp, b"this is not json {{{").unwrap();

    let TileResponse {
        bytes,
        from_cache,
        was_stale,
    } = cache.get_tile(z, x, y).await.unwrap();
    assert_eq!(bytes, b"_tile");
    assert!(from_cache);
    assert!(was_stale);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}

#[tokio::test]
async fn missing_sidecar_treated_as_stale_returns_cached_bytes() {
    // Self-healing path: a crash between `cache::write_atomic`'s step 1
    // (tile) and step 2 (sidecar) leaves "tile + missing sidecar" on
    // disk. The next read MUST treat this as stale and return cached
    // bytes, NOT as a miss (which would refetch synchronously).
    let dir = tempfile::tempdir().unwrap();
    let cache = cache_with_root(dir.path().to_path_buf());

    let z = 8;
    let x = 11;
    let y = 22;
    let tp = pfp_tiles::cache::tile_path(dir.path(), z, x, y);
    std::fs::create_dir_all(tp.parent().unwrap()).unwrap();
    std::fs::write(&tp, b"_orphan_tile").unwrap();
    // No sidecar written.

    let TileResponse {
        bytes,
        from_cache,
        was_stale,
    } = cache.get_tile(z, x, y).await.unwrap();
    assert_eq!(bytes, b"_orphan_tile");
    assert!(from_cache);
    assert!(was_stale);
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
}
