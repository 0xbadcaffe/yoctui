use super::*;

fn fixture() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!(
        "yoctui-rootfs-browser-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(root.join("etc")).unwrap();
    fs::write(root.join("etc/os-release"), "NAME=fixture\n").unwrap();
    fs::write(root.join(".hidden"), "hidden").unwrap();
    fs::canonicalize(root).unwrap()
}

#[test]
fn rootfs_browser_reads_lazy_sorted_entries_and_bounded_text_and_binary() {
    let root = fixture();
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    assert_eq!(entries.len(), 2);
    assert!(entries[0].is_dir);
    assert!(entries[1].is_hidden);
    let children = scan_rootfs_browser_directory(&root, &root.join("etc")).unwrap();
    assert_eq!(children[0].size, Some(13));
    assert_eq!(
        read_rootfs_browser_preview(&root, &children[0].path).unwrap(),
        ("NAME=fixture\n".into(), PreviewKind::Text, false)
    );
    fs::write(root.join("large"), vec![b'x'; 70_000]).unwrap();
    let (text, kind, truncated) = read_rootfs_browser_preview(&root, &root.join("large")).unwrap();
    assert_eq!(text.len(), 64 * 1024);
    assert_eq!(kind, PreviewKind::Text);
    assert!(truncated);
    fs::write(root.join("binary"), b"a\0b").unwrap();
    assert_eq!(
        read_rootfs_browser_preview(&root, &root.join("binary"))
            .unwrap()
            .1,
        PreviewKind::Binary
    );
    assert!(scan_rootfs_browser_directory(&root, &root.join("missing")).is_err());
    assert!(scan_rootfs_browser_directory(&root, root.parent().unwrap()).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn rootfs_browser_preserves_lstat_modes_owners_links_and_never_reads_special_files() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    let root = fixture();
    let file = root.join("etc/os-release");
    fs::set_permissions(&file, fs::Permissions::from_mode(0o4750)).unwrap();
    symlink("etc", root.join("directory-link")).unwrap();
    symlink("/etc/passwd", root.join("outside-link")).unwrap();
    symlink("missing", root.join("dangling-link")).unwrap();
    let fifo = std::ffi::CString::new(root.join("fifo").as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
    let entries = scan_rootfs_browser_directory(&root, &root).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.path.ends_with("dangling-link"))
    );
    for name in ["outside-link", "directory-link", "dangling-link", "fifo"] {
        let entry = entries
            .iter()
            .find(|entry| entry.path.ends_with(name))
            .unwrap();
        assert!(!entry.is_dir);
        assert!(!entry.can_preview());
        assert!(read_rootfs_browser_preview(&root, &entry.path).is_err());
    }
    assert!(scan_rootfs_browser_directory(&root, &root.join("directory-link")).is_err());
    let entry = scan_rootfs_browser_directory(&root, &root.join("etc"))
        .unwrap()
        .remove(0);
    let attributes = entry.rootfs_metadata.unwrap();
    assert_eq!(attributes.mode & 0o7777, 0o4750);
    assert_eq!(attributes.permissions(), "-rwsr-x---");
    assert_eq!(attributes.uid, fs::metadata(&file).unwrap().uid());
    assert_eq!(attributes.gid, fs::metadata(&file).unwrap().gid());
    fs::remove_dir_all(root).unwrap();
}
