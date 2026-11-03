//! Phase 2 -- pfp-state save/load round-trip + edge cases.

use std::path::PathBuf;

use pfp_state::{load_from, save_last_folder_to, save_to, AppState, StateError};

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

#[test]
fn unicode_state_and_photo_paths_survive_repeated_saves() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("앱 상태 📷").join("state.json");
    let mut state = AppState {
        last_folder: Some(tmp.path().join("스캔 사진").join("서울 1990")),
        ..AppState::default()
    };
    save_to(&path, &state).unwrap();
    state.last_pin = Some((37.0, 127.0));
    save_to(&path, &state).unwrap();

    assert_eq!(load_from(&path).unwrap(), state);
    assert_eq!(
        std::fs::read_dir(path.parent().unwrap()).unwrap().count(),
        1
    );
}

#[test]
fn save_last_folder_preserves_pin_in_isolated_state() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("state.json");
    let original = AppState {
        schema_version: 2,
        last_folder: Some(PathBuf::from("/old")),
        last_pin: Some((37.0, 127.0)),
    };
    save_to(&path, &original).unwrap();
    let folder = tmp.path().join("photos");

    save_last_folder_to(&path, &folder).unwrap();

    assert_eq!(
        load_from(&path).unwrap(),
        AppState {
            last_folder: Some(folder),
            ..original
        }
    );
}

#[test]
fn explicit_state_updates_create_missing_file_without_affecting_another_store() {
    let tmp = tempfile::tempdir().unwrap();
    let first = tmp.path().join("first/state.json");
    let second = tmp.path().join("second/state.json");
    let folder = tmp.path().join("photos");
    save_last_folder_to(&first, &folder).unwrap();
    let before = std::fs::read(&first).unwrap();

    pfp_state::save_last_pin_to(&second, 37.0, 127.0).unwrap();

    assert_eq!(std::fs::read(&first).unwrap(), before);
    assert_eq!(
        load_from(&second).unwrap(),
        AppState {
            last_pin: Some((37.0, 127.0)),
            ..AppState::default()
        }
    );
}
