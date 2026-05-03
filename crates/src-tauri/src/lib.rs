//! Tauri host entry. The Builder chain registers Phase 1's two IPC commands
//! (read_exif_summary, list_fixtures). Phase 2/3/4 add to the handler list.
//!
//! D-08 invariant: Tauri-only code lives here. The four pfp-* lib crates carry
//! zero `tauri::*` symbols.

// Plan 02-04: app_state and protocols submodules are populated across Tasks 1-3.
// Task 2 ships the wire DTOs in commands/{folder,state}.rs but the Builder
// chain only references the new commands once Task 3 wires the full chain
// (preserving clippy's dead-code analysis as a real signal in the meantime).
#[allow(dead_code)]
mod app_state;
mod commands;
mod error;
#[allow(dead_code)]
mod protocols;

pub fn run() {
    tauri::Builder::default()
        // Phase 2/3 will add:
        //   .register_asynchronous_uri_scheme_protocol("pfp-thumb", ...)
        //   .register_asynchronous_uri_scheme_protocol("pfp-tile", ...)
        // Hook point preserved here as a comment to make the extension obvious.
        .invoke_handler(tauri::generate_handler![
            commands::exif::read_exif_summary,
            commands::exif::list_fixtures,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    //! Plan 02-04 Task 1 RED gate: pin the new private surface so any later
    //! rename surfaces in the test build.

    use crate::app_state::TauriAppState;
    use crate::error::WireError;

    /// TauriAppState::default() must construct without panicking and seed
    /// the four documented fields. Smoke check; the production Builder calls
    /// the same path inside `.manage(...)`.
    #[test]
    fn tauri_app_state_default_constructs() {
        let s = TauriAppState::default();
        // All four fields must be reachable.
        assert!(s.thumbnail_id_map.lock().unwrap().is_empty());
        assert!(s.in_flight.lock().unwrap().is_empty());
        // Semaphore permits = min(num_cpus, 4); MUST be >= 1 on any host.
        assert!(s.thumb_semaphore.available_permits() >= 1);
        assert!(s.thumb_semaphore.available_permits() <= 4);
        // app_state mirror exposes the canonical default (schema_version = 1).
        let app = s.app_state.lock().unwrap();
        assert_eq!(app.schema_version, 1);
    }

    /// WireError gains `Photos` and `State` variants with `#[serde(tag = "kind")]`.
    /// Construct each once so a future rename surfaces in the test build.
    #[test]
    fn wire_error_has_photos_and_state_variants() {
        let _ = WireError::Photos {
            detail: "x".to_string(),
        };
        let _ = WireError::State {
            detail: "y".to_string(),
        };
    }

    /// `From<pfp_photos::error::PhotosError>` and `From<pfp_state::StateError>`
    /// impls collapse the lib-crate error to the wire `detail` field.
    #[test]
    fn wire_error_from_lib_errors_compiles() {
        // We don't assert details — just that the conversions exist.
        let pe: pfp_photos::error::PhotosError =
            pfp_photos::error::PhotosError::Decode("x".to_string());
        let we: WireError = pe.into();
        assert!(matches!(we, WireError::Photos { .. }));

        let se: pfp_state::StateError = pfp_state::StateError::NoDataLocalDir;
        let we: WireError = se.into();
        assert!(matches!(we, WireError::State { .. }));
    }

    /// Plan 02-04 Task 2 RED gate: pin the wire DTO shapes for folder.rs
    /// and state.rs. The wire DTOs are the IPC contract -- a rename surfaces
    /// here before it surfaces in the Svelte type-import.
    #[test]
    fn folder_wire_dtos_have_expected_shape() {
        use crate::commands::folder::{
            FolderFooter, FolderListing, GpsCoord, PhotoMeta, PhotoSummary,
        };

        let s = PhotoSummary {
            id: "abc".to_string(),
            file_name: "x.jpg".to_string(),
            has_gps: false,
            capture_time: None,
            size_bytes: 0,
            mtime_unix: 0,
        };
        // PhotoSummary must NOT carry absolute_path at the wire boundary.
        // (Compile-time check: only the six fields above are reachable.)
        let _json = serde_json::to_string(&s).unwrap();

        let f = FolderFooter {
            total_jpegs: 0,
            non_image_hidden: 0,
            read_failed: 0,
        };
        let _ = serde_json::to_string(&f).unwrap();

        let l = FolderListing {
            folder_path: "/tmp".to_string(),
            items: vec![],
            footer: f,
            thumb_cache_writable: true,
        };
        let _ = serde_json::to_string(&l).unwrap();

        let m = PhotoMeta {
            id: "abc".to_string(),
            file_name: "x.jpg".to_string(),
            gps: Some(GpsCoord {
                lat: 35.0,
                lng: 139.0,
            }),
            altitude_m: Some(12.5),
            capture_time: Some("2025:01:01 12:00:00".to_string()),
            dimensions: None,
        };
        let _ = serde_json::to_string(&m).unwrap();
    }

    #[test]
    fn state_wire_dtos_have_expected_shape() {
        use crate::commands::state::{AppStateDto, LastFolderStatus};

        // All three variants must construct.
        let n = AppStateDto {
            last_folder: LastFolderStatus::None,
        };
        let s = serde_json::to_string(&n).unwrap();
        // serde tag = "kind", rename_all = "snake_case"
        assert!(s.contains("\"kind\":\"none\""), "got: {s}");

        let a = AppStateDto {
            last_folder: LastFolderStatus::Available {
                path: "/tmp/x".to_string(),
            },
        };
        let s = serde_json::to_string(&a).unwrap();
        assert!(s.contains("\"kind\":\"available\""), "got: {s}");
        assert!(s.contains("\"path\":\"/tmp/x\""), "got: {s}");

        let m = AppStateDto {
            last_folder: LastFolderStatus::Missing {
                path: "/missing".to_string(),
            },
        };
        let s = serde_json::to_string(&m).unwrap();
        assert!(s.contains("\"kind\":\"missing\""), "got: {s}");
    }

    /// Plan 02-04 Task 3 RED gate: pin RequestThumbnailAck wire shape.
    /// `#[serde(tag = "status", rename_all = "snake_case")]` is the IPC
    /// contract for the Svelte frontend's request_thumbnail handler.
    #[test]
    fn request_thumbnail_ack_serializes_with_status_tag() {
        use crate::commands::thumbnail::RequestThumbnailAck;

        let s = serde_json::to_string(&RequestThumbnailAck::Ready).unwrap();
        assert!(s.contains("\"status\":\"ready\""), "got: {s}");

        let s = serde_json::to_string(&RequestThumbnailAck::InFlight).unwrap();
        assert!(s.contains("\"status\":\"in_flight\""), "got: {s}");

        let s = serde_json::to_string(&RequestThumbnailAck::Queued).unwrap();
        assert!(s.contains("\"status\":\"queued\""), "got: {s}");
    }
}
