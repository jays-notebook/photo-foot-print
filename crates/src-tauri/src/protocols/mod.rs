//! Custom URI scheme handlers. Phase 2 ships pfp-thumb://; Phase 3 adds pfp-tile://.
//!
//! D-08 boundary: protocol handlers may import `tauri::*` (this is the host crate),
//! but they delegate all decode / I/O work to lib crates.

pub mod thumb;
pub mod tile;

/// Both native schemes and Windows WebView2 HTTP origins reach handlers in this
/// normalized form: <scheme>://localhost/<path>. Keep paths separate from hosts
/// so convertFileSrc can provide the platform-specific origin without file I/O.
fn local_protocol_path<'a>(uri: &'a http::Uri, scheme: &str) -> Option<&'a str> {
    if uri.scheme_str() != Some(scheme) || uri.authority()?.as_str() != "localhost" {
        return None;
    }
    Some(uri.path())
}
