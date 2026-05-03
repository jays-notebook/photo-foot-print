//! IPC command for the frontend's bootstrap call. Implements D-13 hybrid
//! startup: classify last_folder as None / Available / Missing.

use serde::Serialize;

use crate::error::WireError;

#[derive(Debug, Clone, Serialize)]
pub struct AppStateDto {
    pub last_folder: LastFolderStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LastFolderStatus {
    None,
    Available { path: String },
    Missing { path: String },
}

#[tauri::command]
pub async fn get_app_state() -> Result<AppStateDto, WireError> {
    let state = pfp_state::load()?;
    let last_folder = match state.last_folder {
        None => LastFolderStatus::None,
        Some(p) => {
            let path_str = p.to_string_lossy().into_owned();
            if p.is_dir() {
                LastFolderStatus::Available { path: path_str }
            } else {
                LastFolderStatus::Missing { path: path_str }
            }
        }
    };
    Ok(AppStateDto { last_folder })
}
