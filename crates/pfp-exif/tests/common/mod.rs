use std::path::{Path, PathBuf};

pub fn scanner_fixtures() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/scanner");
    require_fixtures(&root)
}

fn require_fixtures(root: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"))
                && path.metadata().is_ok_and(|m| m.is_file() && m.len() > 1024)
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "No scanner JPEG fixtures in {}. Supply local fixtures per docs/FIXTURES.md; this gate cannot pass without them.", root.display());
    paths
}

#[test]
#[should_panic(expected = "No scanner JPEG fixtures")]
fn missing_fixtures_fail_explicitly() {
    let dir = tempfile::tempdir().unwrap();
    require_fixtures(dir.path());
}
