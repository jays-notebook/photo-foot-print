//! Phase 2 -- list_folder integration test with mixed-content tempdir.

use std::io::Write;
use std::path::PathBuf;

use pfp_photos::list_folder;

fn fixtures_root() -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir)
        .join("..")
        .join("..")
        .join("tests")
        .join("fixtures")
}

fn first_available_jpeg() -> Option<PathBuf> {
    for vendor in ["scanner", "sony", "canon", "nikon"] {
        let dir = fixtures_root().join(vendor);
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("jpg") || s.eq_ignore_ascii_case("jpeg"))
                    .unwrap_or(false)
                    && std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0) > 1024
                {
                    return Some(p);
                }
            }
        }
    }
    None
}

#[test]
fn mixed_content_folder_filters_and_counts_correctly() {
    let Some(src_jpeg) = first_available_jpeg() else {
        eprintln!("[list_folder] No fixture JPEG supplied yet -- skipping.");
        return;
    };
    let tmp = tempfile::tempdir().unwrap();
    let folder = tmp.path();

    // 3 valid JPEG copies (different filenames so cache_key differs)
    for name in ["a.jpg", "b.jpg", "c.JPEG"] {
        std::fs::copy(&src_jpeg, folder.join(name)).unwrap();
    }

    // 1 PNG (wrong extension)
    let mut png = std::fs::File::create(folder.join("graphic.png")).unwrap();
    png.write_all(&[0x89, 0x50, 0x4E, 0x47]).unwrap();
    drop(png);

    // 1 truncated .jpg (right ext, wrong magic)
    let mut bad = std::fs::File::create(folder.join("broken.jpg")).unwrap();
    bad.write_all(&[0x00, 0x01, 0x02, 0x03, 0x04]).unwrap();
    drop(bad);

    // 1 hidden .DS_Store (dotfile)
    std::fs::write(folder.join(".DS_Store"), b"junk").unwrap();

    // 1 nested .pfp-thumbs/ subdirectory (should be skipped by dotfile rule)
    std::fs::create_dir(folder.join(".pfp-thumbs")).unwrap();

    let listing = list_folder(folder).expect("list_folder");

    assert_eq!(listing.items.len(), 3, "expected exactly 3 valid JPEGs");
    assert_eq!(listing.footer.total_jpegs, 3);
    // Hidden: .DS_Store (dotfile) + .pfp-thumbs (dotfile) + graphic.png (wrong ext) + broken.jpg (wrong magic) = 4
    assert_eq!(listing.footer.non_image_hidden, 4, "hidden count");
    assert_eq!(listing.footer.read_failed, 0, "no IO failures expected");

    // Sorted ASC by file_name.
    let names: Vec<String> = listing.items.iter().map(|e| e.file_name.clone()).collect();
    let mut sorted = names.clone();
    sorted.sort();
    assert_eq!(names, sorted, "items must be sorted ASC by file_name");

    // Each item has a 32-char lowercase hex id.
    for item in &listing.items {
        assert_eq!(item.id.len(), 32, "id length");
        assert!(item.id.chars().all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
        assert!(item.absolute_path.starts_with(folder));
    }

    // thumb_cache_writable should be true on a tempdir.
    assert!(listing.thumb_cache_writable);
}

#[test]
fn empty_folder_returns_zero_items() {
    let tmp = tempfile::tempdir().unwrap();
    let listing = list_folder(tmp.path()).expect("list_folder");
    assert_eq!(listing.items.len(), 0);
    assert_eq!(listing.footer.total_jpegs, 0);
    assert_eq!(listing.footer.non_image_hidden, 0);
    assert_eq!(listing.footer.read_failed, 0);
    assert!(listing.thumb_cache_writable);
}

#[test]
fn scanner_fixtures_have_no_gps() {
    // Per D-10: scanner-output JPEGs never have GPS. The list correctly
    // surfaces this through the strict-4 predicate (Plan 02-01).
    let scanner_dir = fixtures_root().join("scanner");
    if !scanner_dir.exists() {
        eprintln!("[list_folder] scanner fixtures dir missing -- skipping.");
        return;
    }
    let listing = list_folder(&scanner_dir).expect("list_folder");
    if listing.items.is_empty() {
        eprintln!("[list_folder] scanner fixtures dir empty -- skipping.");
        return;
    }
    for item in &listing.items {
        assert!(!item.has_gps, "scanner fixture {} unexpectedly has GPS", item.file_name);
    }
}
