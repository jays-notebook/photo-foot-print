//! Fixture-free atomic replacement regressions, including Windows file sharing.

use std::path::Path;

use pfp_exif::atomic::write_via_temp;

fn assert_no_temporary_files(folder: &Path) {
    let orphans: Vec<_> = std::fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .filter(|name| name.to_string_lossy().starts_with(".pfp-tmp-"))
        .collect();
    assert!(orphans.is_empty(), "temporary files remain: {orphans:?}");
}

#[test]
fn creates_and_replaces_a_photo_in_a_unicode_folder() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("스캔 사진 📷");
    std::fs::create_dir(&folder).unwrap();
    let target = folder.join("서울 필름 사진.jpg");

    write_via_temp(&target, |path| std::fs::write(path, b"first version")).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"first version");
    write_via_temp(&target, |path| {
        assert_eq!(std::fs::read(path)?, b"first version");
        std::fs::write(path, b"second version")
    })
    .unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"second version");
    assert_no_temporary_files(&folder);
}

#[test]
fn failed_writer_preserves_original_and_removes_partial_temporary_file() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("photo.jpg");
    std::fs::write(&target, b"original").unwrap();

    let error = write_via_temp(&target, |path| {
        std::fs::write(path, b"partial replacement")?;
        Err(std::io::Error::other("writer failed"))
    })
    .unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::Other);
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    assert_no_temporary_files(dir.path());
}

#[cfg(windows)]
#[test]
fn target_without_delete_sharing_is_preserved_until_the_lock_is_released() {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_READ, FILE_SHARE_WRITE};

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("잠긴 사진.jpg");
    std::fs::write(&target, b"original").unwrap();
    let locked = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
        .open(&target)
        .unwrap();

    assert!(write_via_temp(&target, |path| std::fs::write(path, b"replacement")).is_err());
    assert_eq!(std::fs::read(&target).unwrap(), b"original");
    assert_no_temporary_files(dir.path());

    drop(locked);
    write_via_temp(&target, |path| std::fs::write(path, b"replacement")).unwrap();
    assert_eq!(std::fs::read(&target).unwrap(), b"replacement");
    assert_no_temporary_files(dir.path());
}

#[cfg(windows)]
#[test]
fn embedded_nul_cannot_redirect_the_replacement_to_a_different_target() {
    let dir = tempfile::tempdir().unwrap();
    let existing = dir.path().join("photo.jpg");
    std::fs::write(&existing, b"original").unwrap();
    let invalid_target = dir.path().join("photo.jpg\0ignored");

    let error =
        write_via_temp(&invalid_target, |path| std::fs::write(path, b"replacement")).unwrap_err();

    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(std::fs::read(&existing).unwrap(), b"original");
    assert_no_temporary_files(dir.path());
}

#[cfg(windows)]
#[test]
fn committed_file_is_not_marked_temporary() {
    use std::os::windows::fs::MetadataExt;
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_TEMPORARY;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("photo.jpg");
    write_via_temp(&target, |path| std::fs::write(path, b"replacement")).unwrap();

    assert_eq!(
        std::fs::metadata(&target).unwrap().file_attributes() & FILE_ATTRIBUTE_TEMPORARY,
        0
    );
}

#[cfg(windows)]
#[test]
fn replacement_preserves_hidden_and_not_content_indexed_attributes() {
    use std::os::windows::{ffi::OsStrExt, fs::MetadataExt};
    use windows_sys::Win32::Storage::FileSystem::{
        SetFileAttributesW, FILE_ATTRIBUTE_HIDDEN, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED,
        FILE_ATTRIBUTE_TEMPORARY,
    };

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("hidden photo.jpg");
    std::fs::write(&target, b"original").unwrap();
    let retained = FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
    let wide: Vec<_> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // SAFETY: wide contains the live, NUL-terminated path of our test file.
    assert_ne!(unsafe { SetFileAttributesW(wide.as_ptr(), retained) }, 0);

    write_via_temp(&target, |path| std::fs::write(path, b"replacement")).unwrap();

    let attributes = std::fs::metadata(&target).unwrap().file_attributes();
    assert_eq!(attributes & retained, retained);
    assert_eq!(attributes & FILE_ATTRIBUTE_TEMPORARY, 0);
    assert_eq!(std::fs::read(&target).unwrap(), b"replacement");
    assert_no_temporary_files(dir.path());
}

#[cfg(windows)]
#[test]
fn read_only_target_is_preserved_without_leaving_a_read_only_temporary_file() {
    use std::os::windows::fs::MetadataExt;

    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("photo.jpg");
    std::fs::write(&target, b"original").unwrap();
    let original_permissions = std::fs::metadata(&target).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    std::fs::set_permissions(&target, permissions).unwrap();
    let attributes_before = std::fs::metadata(&target).unwrap().file_attributes();

    let result = write_via_temp(&target, |path| std::fs::write(path, b"replacement"));
    let actual = std::fs::read(&target).unwrap();
    let attributes_after = std::fs::metadata(&target).unwrap().file_attributes();
    // Restore writability before assertions so the test directory can be removed.
    std::fs::set_permissions(&target, original_permissions).unwrap();

    assert!(result.is_err());
    assert_eq!(actual, b"original");
    assert_eq!(attributes_after, attributes_before);
    assert_no_temporary_files(dir.path());
}
