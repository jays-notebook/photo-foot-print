//! Phase 1 IPC smoke test (headless).
//!
//! Per ARCHITECTURE.md §"Testing Seams" / RESEARCH.md Pattern 1, the
//! `#[tauri::command]` wrappers are 5–15 lines that just delegate to pfp-exif.
//! We unit-test pfp-exif directly (Plan 02). Here we just validate that the
//! command body's *logic* — list fixtures, read summary, return DTO shape —
//! works end-to-end against any user-supplied real fixture, without spinning
//! up a webview.
//!
//! The actual webview-launch verification is the human-verify checkpoint
//! later in this plan.

use std::path::PathBuf;

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

#[test]
fn read_summary_works_on_supplied_fixtures() {
    // Walk fixture vendors. If none are supplied (fresh clone), the test no-ops.
    let mut found_any = false;
    for vendor in ["sony", "canon", "nikon"] {
        let p = fixtures_root().join(vendor).join("sample.jpg");
        if !p.exists() || std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) <= 1024 {
            continue;
        }
        found_any = true;

        // This is the same call the IPC command makes.
        let summary = pfp_exif::read_summary(&p)
            .unwrap_or_else(|e| panic!("[{vendor}] read_summary failed: {e:?}"));

        // SummaryDto would be constructed here by the command. We exercise the
        // shape implicitly by reading the fields.
        let _ = summary.has_gps;
        let _ = summary.capture_time.clone();
    }
    if !found_any {
        eprintln!("No fixtures supplied — see docs/FIXTURES.md. ipc_smoke noop.");
    }
}

/// Phase 2 lib-crate smoke: exercise the path Plan 04's `list_folder` and
/// `read_photo_meta` IPC commands delegate to (`pfp_photos::list_folder` +
/// `pfp_exif::read_detail`) without spinning up a webview.
///
/// The IPC command bodies themselves are not tested here -- consistent with
/// Phase 1's testing seam (RESEARCH §Question 10 "What we deliberately DO
/// NOT test").
#[test]
fn phase_2_list_folder_smoke_scanner_fixtures() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let scanner_dir = std::path::PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
        .join("scanner");

    if !scanner_dir.exists() {
        eprintln!("[phase_2_list_folder_smoke] scanner fixtures dir missing -- skipping.");
        return;
    }

    let listing = pfp_photos::list_folder(&scanner_dir).expect("list_folder");
    if listing.items.is_empty() {
        eprintln!("[phase_2_list_folder_smoke] scanner fixtures dir empty -- skipping.");
        return;
    }

    // D-10: scanner fixtures never have GPS.
    for item in &listing.items {
        assert!(
            !item.has_gps,
            "scanner fixture {} unexpectedly has GPS",
            item.file_name
        );
        assert_eq!(item.id.len(), 32, "id length");
    }

    // Per-item read_detail also yields gps == None.
    let first = &listing.items[0];
    let detail = pfp_exif::read_detail(&first.absolute_path).expect("read_detail");
    assert!(
        detail.gps.is_none(),
        "detail.gps must be None for scanner fixtures"
    );
}
