//! Typed errors for the tile cache + fetch pipeline. D-08: no `tauri::*`.
//!
//! TileError does NOT cross the IPC boundary in Phase 3 -- tile errors map
//! to HTTP status codes inside `protocols/tile.rs` (200 hit / 502 miss).
//! Hence no `From<TileError> for WireError` impl is needed in Phase 3.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum TileError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("upstream tile fetch failed: {0}")]
    Upstream(String),

    #[error("upstream returned status {0}")]
    UpstreamStatus(u16),

    #[error("sidecar (de)serialization: {0}")]
    Sidecar(String),

    #[error("reqwest client init: {0}")]
    Init(String),
}
