//! IPC command modules. Each #[tauri::command] is a thin wrapper that
//! delegates into the lib crates (Pattern 1: Lib-First / Tauri-Thin).
//!
//! D-08: this directory and `crate::protocols` are the ONLY places where
//! `tauri::*` symbols meet pfp_* lib types.

pub mod exif; // Phase 1 — kept for ipc_smoke regression test.
pub mod folder; // Phase 2 — open_folder_dialog, list_folder, read_photo_meta.
pub mod state; // Phase 2 — get_app_state.
pub mod thumbnail; // Phase 2 — request_thumbnail (emits thumbnail-ready event).
