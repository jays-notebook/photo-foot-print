//! pfp-tile:// async URI scheme handler. Serves OSM tile bytes from the
//! pfp-tiles disk cache, with stale-while-revalidate against upstream
//! tile.openstreetmap.org (User-Agent set by pfp-tiles per OSMF policy).
//!
//! Wire shape: pfp-tile://osm/{z}/{x}/{y}.png
//!   200 image/png + cache-control on hit (fresh OR stale-served)
//!   400 on malformed URL (bad host, bad z/x/y, missing .png, z>19, extra path)
//!   502 on cache miss + upstream fetch failure (D-34)
//!   503 if the tile_fetch_semaphore is closed (should never happen)
//!
//! Pitfalls: 1 (User-Agent), 3 (CSP), 5 (keepBuffer enforced client-side
//! in Plan 04), 6 (sidecar absent vs parse — pfp_tiles handles this).

use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder};

use crate::app_state::TauriAppState;

/// Build + send an HTTP response. Centralized to mirror protocols/thumb.rs.
fn respond(
    responder: UriSchemeResponder,
    status: u16,
    content_type: Option<&'static str>,
    cache_control: Option<&'static str>,
    body: Vec<u8>,
) {
    let mut builder = http::Response::builder().status(status);
    if let Some(ct) = content_type {
        builder = builder.header("Content-Type", ct);
    }
    if let Some(cc) = cache_control {
        builder = builder.header("Cache-Control", cc);
    }
    let resp = builder.body(body).unwrap_or_else(|_| {
        http::Response::builder()
            .status(500)
            .body(Vec::new())
            .expect("infallible: 500 response with empty body")
    });
    responder.respond(resp);
}

/// Parse "/z/x/y.png" → (z, x, y). Strict: rejects missing leading "/",
/// rejects non-numeric components, rejects missing ".png" suffix, rejects
/// out-of-range zoom (0..=19), rejects extra path segments.
pub(crate) fn parse_zxy(path: &str) -> Option<(u8, u32, u32)> {
    let path = path.strip_prefix('/')?;
    let path = path.strip_suffix(".png")?;
    let mut parts = path.splitn(4, '/');
    let z: u8 = parts.next()?.parse().ok()?;
    let x: u32 = parts.next()?.parse().ok()?;
    let y: u32 = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if z > 19 {
        return None;
    }
    Some((z, x, y))
}

pub fn handle<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app: AppHandle<R> = ctx.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let url = request.uri();

        // Delta vs thumb.rs: host MUST be "osm" (forward-compat for future
        // tile providers — never mix per-provider sidecars).
        let host = url.host().unwrap_or("");
        if host != "osm" {
            respond(responder, 400, None, None, Vec::new());
            return;
        }

        // Delta vs thumb.rs: parse path as /z/x/y.png with strict checks.
        let path = url.path();
        let (z, x, y) = match parse_zxy(path) {
            Some(t) => t,
            None => {
                respond(responder, 400, None, None, Vec::new());
                return;
            }
        };

        let state: tauri::State<'_, TauriAppState> = app.state();

        // Acquire semaphore permit before delegating to TileCache. Per
        // RESEARCH §Pattern 3 the permit is held across get_tile only;
        // cache hits return fast and free the permit immediately.
        let permit = match state.tile_fetch_semaphore.clone().acquire_owned().await {
            Ok(p) => p,
            Err(_) => {
                // Semaphore closed — should never happen during normal operation.
                respond(responder, 503, None, None, Vec::new());
                return;
            }
        };
        let result = state.tiles.get_tile(z, x, y).await;
        drop(permit);

        match result {
            Ok(tile) => {
                respond(
                    responder,
                    200,
                    Some("image/png"),
                    // Long max-age — webview-side caching helps avoid even the
                    // pfp-tile:// roundtrip on repeated paint cycles. The Rust-side
                    // freshness machinery is the source of truth.
                    Some("max-age=86400"),
                    tile.bytes,
                );
            }
            Err(_e) => {
                // D-34: on uncached miss + upstream failure, return 502.
                // Leaflet's default tileerror handler renders blank.
                respond(responder, 502, None, None, Vec::new());
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_zxy_happy_path() {
        assert_eq!(parse_zxy("/14/13708/6334.png"), Some((14, 13708, 6334)));
    }

    #[test]
    fn parse_zxy_zoom_zero_ok() {
        assert_eq!(parse_zxy("/0/0/0.png"), Some((0, 0, 0)));
    }

    #[test]
    fn parse_zxy_zoom_nineteen_ok() {
        assert_eq!(parse_zxy("/19/100/100.png"), Some((19, 100, 100)));
    }

    #[test]
    fn parse_zxy_rejects_zoom_above_nineteen() {
        assert!(parse_zxy("/20/0/0.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_missing_png_suffix() {
        assert!(parse_zxy("/14/13708/6334").is_none());
    }

    #[test]
    fn parse_zxy_rejects_missing_leading_slash() {
        assert!(parse_zxy("14/13708/6334.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_non_numeric_z() {
        assert!(parse_zxy("/abc/13708/6334.png").is_none());
    }

    #[test]
    fn parse_zxy_rejects_extra_segment() {
        assert!(parse_zxy("/14/13708/6334.png/extra").is_none());
    }

    #[test]
    fn parse_zxy_rejects_path_traversal() {
        // .. components do not bypass strict numeric parsing.
        assert!(parse_zxy("/../1/1.png").is_none());
        assert!(parse_zxy("/14/../1.png").is_none());
    }
}
