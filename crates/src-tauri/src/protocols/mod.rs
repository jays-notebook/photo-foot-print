//! Custom URI scheme handlers. Phase 2 ships pfp-thumb://; Phase 3 will add pfp-tile://.
//!
//! D-08 boundary: protocol handlers may import `tauri::*` (this is the host crate),
//! but they delegate all decode / I/O work to lib crates.

pub mod thumb;
