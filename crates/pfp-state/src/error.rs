//! Typed errors for the state persistence layer. Designed to cross into
//! src-tauri's WireError (D-08: this crate has no `tauri::*`).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum StateError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("state.json parse error: {0}")]
    Parse(String),

    #[error("dirs::data_local_dir() returned None — cannot resolve state.json location")]
    NoDataLocalDir,

    #[error("state path has no parent directory")]
    NoParent,
}
