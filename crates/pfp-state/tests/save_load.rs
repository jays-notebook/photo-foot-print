//! Phase 2 -- pfp-state save/load round-trip + edge cases.

use std::path::PathBuf;

use pfp_state::{load_from, save_last_folder, save_to, AppState, StateError};

#[test]
fn round_trip_preserves_all_fields() {
    let tmp = tempfile::tempdir().unwrap();
    let state_path = tmp.path().join("state.json");
    let original = AppState {
        schema_version: 1,
        last_folder: Some(PathBuf::from("/tmp/scans-2026-05")),
        last_pin: None,
    };
    save_to(&state_path, &original).expect("save");
    let loaded = load_from(&state_path).expect("load");
    assert_eq!(loaded, original);
}

#[test]
fn load_missing_returns_default() {
    let tmp = tempfile::tempdir().unwrap();
    let nonexistent = tmp.path().join("does-not-exist.json");
    let loaded = load_from(&nonexistent).expect("load missing");
    assert_eq!(loaded, AppState::default());
    assert_eq!(loaded.schema_version, 1);
    assert!(loaded.last_folder.is_none());
}

#[test]
fn load_malformed_returns_parse_error() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("bad.json");
    std::fs::write(&path, b"{ this is not valid JSON ::: ").unwrap();
    let err = load_from(&path).unwrap_err();
    match err {
        StateError::Parse(_) => {}
        other => panic!("expected Parse, got {other:?}"),
    }
}

#[test]
fn load_partial_uses_serde_default_for_schema_version() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("partial.json");
    // No schema_version field -- serde must apply the default function.
    std::fs::write(&path, br#"{"last_folder":"/tmp/scans"}"#).unwrap();
    let loaded = load_from(&path).expect("load partial");
    assert_eq!(
        loaded.schema_version, 1,
        "serde default must populate schema_version"
    );
    assert_eq!(loaded.last_folder, Some(PathBuf::from("/tmp/scans")));
}

#[test]
fn load_partial_with_unknown_field_ignored() {
    // Forward-compat probe: an unknown future field should not break loading.
    // (serde default behaviour is "ignore unknown fields".)
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("future.json");
    std::fs::write(
        &path,
        br#"{"schema_version":2,"last_folder":"/tmp/x","last_pin":[35.0,139.0]}"#,
    )
    .unwrap();
    let loaded = load_from(&path).expect("load future");
    assert_eq!(loaded.schema_version, 2);
    assert_eq!(loaded.last_folder, Some(PathBuf::from("/tmp/x")));
}

#[test]
fn save_to_creates_parent_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let nested = tmp.path().join("a").join("b").join("c").join("state.json");
    let state = AppState {
        schema_version: 1,
        last_folder: Some(PathBuf::from("/tmp/y")),
        last_pin: None,
    };
    save_to(&nested, &state).expect("save creates parents");
    assert!(nested.exists());
    let loaded = load_from(&nested).expect("load");
    assert_eq!(loaded, state);
}

#[test]
fn save_overwrites_existing() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("state.json");
    save_to(
        &path,
        &AppState {
            schema_version: 1,
            last_folder: Some(PathBuf::from("/old")),
            last_pin: None,
        },
    )
    .unwrap();
    save_to(
        &path,
        &AppState {
            schema_version: 1,
            last_folder: Some(PathBuf::from("/new")),
            last_pin: None,
        },
    )
    .unwrap();
    let loaded = load_from(&path).unwrap();
    assert_eq!(loaded.last_folder, Some(PathBuf::from("/new")));
}

/// This test exercises the production `save_last_folder` (uses dirs::data_local_dir).
/// We can't isolate the canonical path easily in a test, so we just smoke-test
/// that the function runs without error AND that load() roundtrips.
///
/// The test resets `last_folder = None` at the end. The plan explicitly
/// permits this -- it is acceptable for a personal-use tool. If it becomes
/// a problem (CI, multiple parallel agents) the plan authorizes
/// `#[ignore]` as the fallback.
#[test]
fn save_last_folder_uses_canonical_path() {
    let test_folder =
        std::env::temp_dir().join(format!("pfp-state-test-{}", std::process::id()));
    std::fs::create_dir_all(&test_folder).unwrap();

    save_last_folder(&test_folder).expect("save_last_folder");
    let loaded = pfp_state::load().expect("load");
    assert_eq!(
        loaded.last_folder.as_deref().map(|p| p.to_path_buf()),
        Some(test_folder.clone()),
        "save_last_folder must persist into state.json read by load()"
    );

    // Cleanup -- restore to a known state to avoid polluting subsequent runs.
    let mut restored = loaded;
    restored.last_folder = None;
    pfp_state::save(&restored).expect("cleanup save");
    let _ = std::fs::remove_dir(&test_folder);
}
