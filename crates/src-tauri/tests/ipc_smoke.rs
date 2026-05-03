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
