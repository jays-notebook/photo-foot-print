//! Phase 2 placeholder — implementation lands in Plan 04 Task 2.
use crate::error::WireError;

#[tauri::command]
pub async fn get_app_state() -> Result<serde_json::Value, WireError> {
    unimplemented!("Plan 04 Task 2")
}
