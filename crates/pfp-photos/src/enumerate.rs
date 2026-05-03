//! Phase 2 placeholder -- real implementation lands in Plan 02 Task 3.

use std::path::Path;
use serde::Serialize;
use crate::error::PhotosError;

#[derive(Debug, Clone, Serialize)]
pub struct PhotoEntry { pub _placeholder: () }

#[derive(Debug, Clone, Serialize, Default)]
pub struct FolderFooter {
    pub total_jpegs: usize,
    pub non_image_hidden: usize,
    pub read_failed: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct FolderListing {
    pub folder_path: String,
    pub items: Vec<PhotoEntry>,
    pub footer: FolderFooter,
    pub thumb_cache_writable: bool,
}

pub fn list_folder(_folder: &Path) -> Result<FolderListing, PhotosError> {
    unimplemented!("Plan 02 Task 3")
}
