//! pfp-thumb:// async URI scheme handler. Serves cached thumbnail bytes
//! directly to <img src=...> without crossing the JSON IPC boundary.
//!
//! Wire shape: pfp-thumb://<id>   (id is the 32-hex-char cache_key)
//!   200 image/jpeg + immutable cache header on cache hit
//!   404 on cache miss (frontend retries after thumbnail-ready event)
//!   400 on missing host
//!   500 reserved for internal errors (lock poisoning, missing parent path)
//!
//! Pitfalls: 2-B (always set Content-Type), 2-E (no path canonicalization
//! attempt — the id is opaque, the id-map is the authority).

use std::path::PathBuf;

use tauri::{AppHandle, Manager, UriSchemeContext, UriSchemeResponder};

use crate::app_state::TauriAppState;

/// Build + send an HTTP response. WR-05: every prior call site used
/// `.body(...).unwrap()`, which panics if `http::response::Builder`
/// accumulated an internal error (for current call sites it doesn't --
/// only literal status codes / headers are passed -- but the pattern
/// invites future contributors to copy a panic into a Tokio task that
/// would silently abort and hang the requesting `<img>`). Centralize
/// the build + on-`Err` fall back to a hard-coded 500.
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
        // Fallback: build a minimal 500 with NO headers. The Builder cannot
        // fail when given only a numeric status and an empty body, so this
        // expect() is genuinely infallible -- but we name the call site so
        // any future change that breaks the invariant points at the right
        // line.
        http::Response::builder()
            .status(500)
            .body(Vec::new())
            .expect("infallible: 500 response with empty body")
    });
    responder.respond(resp);
}

pub fn handle<R: tauri::Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: http::Request<Vec<u8>>,
    responder: UriSchemeResponder,
) {
    let app: AppHandle<R> = ctx.app_handle().clone();
    tauri::async_runtime::spawn(async move {
        let url = request.uri();
        // Parse "pfp-thumb://<host>". Use the host segment as the id.
        let id = match url.host() {
            Some(h) => h.to_string(),
            None => {
                respond(responder, 400, None, None, Vec::new());
                return;
            }
        };

        // Resolve id -> PathBuf via the managed-state id-map.
        let state: tauri::State<'_, TauriAppState> = app.state();
        let path: PathBuf = {
            let map = match state.thumbnail_id_map.lock() {
                Ok(m) => m,
                Err(_) => {
                    respond(responder, 500, None, None, Vec::new());
                    return;
                }
            };
            match map.get(&id).cloned() {
                Some(p) => p,
                None => {
                    respond(responder, 404, None, None, Vec::new());
                    return;
                }
            }
        };

        let folder = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                respond(responder, 500, None, None, Vec::new());
                return;
            }
        };

        match pfp_photos::cache::read_cached_thumbnail(&folder, &id) {
            Ok(bytes) => {
                respond(
                    responder,
                    200,
                    Some("image/jpeg"),
                    // The cache key encodes mtime + size; the file content
                    // is immutable for a given URL. Aggressive cache.
                    Some("max-age=31536000, immutable"),
                    bytes,
                );
            }
            Err(_not_yet_cached) => {
                respond(responder, 404, None, None, Vec::new());
            }
        }
    });
}
