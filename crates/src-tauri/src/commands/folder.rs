//! Phase 2 placeholder — implementation lands in Plan 04 Task 2.
use crate::error::WireError;

#[tauri::command]
pub async fn open_folder_dialog(_app: tauri::AppHandle) -> Result<Option<String>, WireError> {
    unimplemented!("Plan 04 Task 2")
}

#[tauri::command]
pub async fn list_folder(
    _state: tauri::State<'_, crate::app_state::TauriAppState>,
    _path: String,
) -> Result<serde_json::Value, WireError> {
    unimplemented!("Plan 04 Task 2")
}

#[tauri::command]
pub async fn read_photo_meta(
    _state: tauri::State<'_, crate::app_state::TauriAppState>,
    _id: String,
) -> Result<serde_json::Value, WireError> {
    unimplemented!("Plan 04 Task 2")
}
