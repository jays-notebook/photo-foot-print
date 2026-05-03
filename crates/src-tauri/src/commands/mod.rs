//! IPC command modules. Each #[tauri::command] is a thin wrapper that
//! delegates into the lib crates (Pattern 1: Lib-First / Tauri-Thin).
//!
//! D-08: this directory and `crate::protocols` are the ONLY places where
//! `tauri::*` symbols meet pfp_* lib types.

pub mod exif; // Phase 1 — kept for ipc_smoke regression test.

// Plan 02-04 Phase 2 commands. The dead-code allow is removed by Task 3
// (when the Builder chain references each command in `tauri::generate_handler!`).
#[allow(dead_code)]
pub mod folder; // Phase 2 — open_folder_dialog, list_folder, read_photo_meta.
#[allow(dead_code)]
pub mod state; // Phase 2 — get_app_state.
#[allow(dead_code)]
pub mod thumbnail; // Phase 2 — request_thumbnail (emits thumbnail-ready event).
