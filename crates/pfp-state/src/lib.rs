//! pfp-state — Reserved for Phase 4.
//!
//! Phase 4 will add: last-pin / recent-folder / prefs JSON state under app_data_dir().
//! No tauri::* symbols allowed (D-08).

#[cfg(test)]
mod tests {
    use crate::{AppState, StateError};

    /// AppState::default() must yield schema_version = 1 (NOT the derived 0).
    /// Plan 02-03 Task 1 RED gate.
    #[test]
    fn default_app_state_has_schema_version_1_and_no_last_folder() {
        let s = AppState::default();
        assert_eq!(s.schema_version, 1);
        assert!(s.last_folder.is_none());
    }

    /// state_path() must end in photo-foot-print/state.json under data_local_dir().
    #[test]
    fn state_path_ends_with_app_dir_and_state_json() {
        let p = crate::state_path().expect("state_path resolves on dev machine");
        let s = p.to_string_lossy();
        assert!(
            s.ends_with("photo-foot-print/state.json")
                || s.ends_with("photo-foot-print\\state.json"),
            "unexpected state_path tail: {s}",
        );
    }

    /// StateError::NoDataLocalDir is the documented failure mode for state_path()
    /// when dirs::data_local_dir() returns None. Compile-only smoke check.
    #[test]
    fn state_error_has_no_data_local_dir_variant() {
        // Construct each variant once so a future rename surfaces in the test build.
        let _ = StateError::NoDataLocalDir;
        let _ = StateError::NoParent;
        let _ = StateError::Parse("x".to_string());
    }
}
