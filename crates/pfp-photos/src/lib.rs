//! pfp-photos -- folder enumeration + JPEG validity + thumbnail pipeline.
//!
//! D-08 invariant: zero `tauri::*` symbols. Pure Rust on top of `image`,
//! `fast_image_resize`, `sha2`, `pfp-exif`.
//! Used by:
//!   - `crates/src-tauri/src/commands/folder.rs`    (Plan 04) -- thin IPC wrappers for list_folder + read_photo_meta
//!   - `crates/src-tauri/src/commands/thumbnail.rs` (Plan 04) -- async thumbnail decode queue
//!   - `crates/src-tauri/src/protocols/thumb.rs`    (Plan 04) -- pfp-thumb:// URI handler reads cached bytes
//!
//! Architectural rules:
//!   - All paths in / out are absolute (callers canonicalize at IPC boundary).
//!   - Filter pipeline (Pitfall 2-D, RESEARCH §Question 8): dotfile -> extension -> magic byte -> metadata -> read_summary.
//!   - Cache directory is folder-local: <folder>/.pfp-thumbs/ (D-17). NEVER falls back silently to system cache (D-18).
//!   - Atomic cache writes: sibling tempfile + sync + persist + parent fsync. NO F_FULLFSYNC (regenerable, per atomic.rs note in PATTERNS).

pub mod cache;
pub mod enumerate;
pub mod error;
pub mod jpeg;
pub mod thumbnail;

pub use crate::enumerate::{list_folder, FolderFooter, FolderListing, PhotoEntry};
pub use crate::error::PhotosError;
