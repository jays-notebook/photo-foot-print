//! Tauri host entry. Builder chain registers Phase 1 commands + Phase 2's
//! folder/photo/state IPC + the pfp-thumb:// async URI scheme.
//!
//! D-08 invariant: Tauri-only code lives here, in `commands/`, and `protocols/`.
//! The four pfp-* lib crates carry zero `tauri::*` symbols.

mod app_state;
mod commands;
mod error;
mod protocols;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .register_asynchronous_uri_scheme_protocol("pfp-thumb", protocols::thumb::handle)
        // Phase 3: register pfp-tile:// — same shape as pfp-thumb://.
        .register_asynchronous_uri_scheme_protocol("pfp-tile", protocols::tile::handle)
        // Phase 3: replace `.manage(TauriAppState::default())` with a `.setup`
        // hook that resolves app_cache_dir() and constructs TileCache once.
        .setup(|app| {
            use tauri::Manager;
            // Resolve cache_root via app.path().app_cache_dir() (Phase 3 D-30).
            let resolved = app.path().app_cache_dir().map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;
            let cache_root = resolved.join("tiles");
            let http = pfp_tiles::build_osm_client().map_err(|e| {
                Box::new(std::io::Error::other(e.to_string())) as Box<dyn std::error::Error>
            })?;
            let tiles = pfp_tiles::TileCache::new(cache_root, http);
            // Plan 04-03 Task 1 GREEN: with_tiles widened to accept
            // initial_last_pin. Task 2 will wire pfp_state::load() to
            // extract the real boot-loaded pin; until then pass None
            // so the lib compiles. (Two-task split: signature change
            // lands first, .setup wiring lands second.)
            let state = app_state::TauriAppState::with_tiles(tiles, None);
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Phase 1 commands (kept — ipc_smoke depends on them).
            commands::exif::read_exif_summary,
            commands::exif::list_fixtures,
            // Phase 2 commands.
            commands::folder::open_folder_dialog,
            commands::folder::list_folder,
            commands::folder::read_photo_meta,
            commands::thumbnail::request_thumbnail,
            commands::state::get_app_state,
            // Phase 3 commands.
            commands::geotag::save_geotag,
            commands::geotag::get_session_last_pin,
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

    /// Plan 03-03 Task 3 RED gate: pin the new WireError variant + serde shape.
    #[test]
    fn wire_error_exif_write_variant_serializes() {
        let we = WireError::ExifWrite {
            detail: "boom".to_string(),
        };
        let s = serde_json::to_string(&we).unwrap();
        assert!(s.contains("\"kind\":\"exif_write\""), "got: {s}");
        assert!(s.contains("\"detail\":\"boom\""), "got: {s}");
    }

    /// Plan 03-03 Task 3 RED gate (extended for Plan 04-03 Task 1):
    /// pin TauriAppState::with_tiles seeds the Phase 3 fields correctly
    /// AND honours the Phase 4 D-46 widened signature
    /// `with_tiles(tiles, initial_last_pin: Option<(f64, f64)>)`.
    #[test]
    fn tauri_app_state_with_tiles_seeds_phase3_fields() {
        use crate::app_state::TauriAppState;

        // Phase 4 (D-46): Some(...) seed lands in session_last_pin Mutex.
        let tiles_some = pfp_tiles::TileCache::new(
            std::env::temp_dir().join("pfp-with-tiles-test-some"),
            pfp_tiles::build_osm_client().unwrap(),
        );
        let s_some = TauriAppState::with_tiles(tiles_some, Some((35.0, 139.0)));
        assert_eq!(
            *s_some.session_last_pin.lock().unwrap(),
            Some((35.0, 139.0)),
            "Some((35.0, 139.0)) seed must land in session_last_pin"
        );
        // tile_fetch_semaphore has 4 permits per D-35 (regression guard).
        assert_eq!(s_some.tile_fetch_semaphore.available_permits(), 4);

        // Phase 3 baseline regression: None seed → session_last_pin == None.
        let tiles_none = pfp_tiles::TileCache::new(
            std::env::temp_dir().join("pfp-with-tiles-test-none"),
            pfp_tiles::build_osm_client().unwrap(),
        );
        let s_none = TauriAppState::with_tiles(tiles_none, None);
        assert!(
            s_none.session_last_pin.lock().unwrap().is_none(),
            "None seed must leave session_last_pin as None"
        );
        assert_eq!(s_none.tile_fetch_semaphore.available_permits(), 4);
    }
}
