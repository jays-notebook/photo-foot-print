//! Phase 2 placeholder — implementation lands in Plan 04 Task 3.
use crate::error::WireError;

#[tauri::command]
pub async fn request_thumbnail(
    _state: tauri::State<'_, crate::app_state::TauriAppState>,
    _app: tauri::AppHandle,
    _id: String,
) -> Result<serde_json::Value, WireError> {
    unimplemented!("Plan 04 Task 3")
}
