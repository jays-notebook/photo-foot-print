//! Tauri host entry. The Builder chain registers Phase 1's two IPC commands
//! (read_exif_summary, list_fixtures). Phase 2/3/4 add to the handler list.
//!
//! D-08 invariant: Tauri-only code lives here. The four pfp-* lib crates carry
//! zero `tauri::*` symbols.

mod app_state;
mod commands;
mod error;
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
}
