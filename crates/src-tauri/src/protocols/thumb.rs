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
                responder.respond(
                    http::Response::builder()
                        .status(400)
                        .body(Vec::new())
                        .unwrap(),
                );
                return;
            }
        };

        // Resolve id -> PathBuf via the managed-state id-map.
        let state: tauri::State<'_, TauriAppState> = app.state();
        let path: PathBuf = {
            let map = match state.thumbnail_id_map.lock() {
                Ok(m) => m,
                Err(_) => {
                    responder.respond(
                        http::Response::builder()
                            .status(500)
                            .body(Vec::new())
                            .unwrap(),
                    );
                    return;
                }
            };
            match map.get(&id).cloned() {
                Some(p) => p,
                None => {
                    responder.respond(
                        http::Response::builder()
                            .status(404)
                            .body(Vec::new())
                            .unwrap(),
                    );
                    return;
                }
            }
        };

        let folder = match path.parent() {
            Some(p) => p.to_path_buf(),
            None => {
                responder.respond(
                    http::Response::builder()
                        .status(500)
                        .body(Vec::new())
                        .unwrap(),
                );
                return;
            }
        };

        match pfp_photos::cache::read_cached_thumbnail(&folder, &id) {
            Ok(bytes) => {
                responder.respond(
                    http::Response::builder()
                        .status(200)
                        .header("Content-Type", "image/jpeg")
                        // The cache key encodes mtime + size; the file content
                        // is immutable for a given URL. Aggressive cache.
                        .header("Cache-Control", "max-age=31536000, immutable")
                        .body(bytes)
                        .unwrap(),
                );
            }
            Err(_not_yet_cached) => {
                responder.respond(
                    http::Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .unwrap(),
                );
            }
        }
    });
}
