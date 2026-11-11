use super::*;

#[test]
fn environment_setup_script_scan_lists_extensionless_nonexecutables_and_symlinks() {
    use std::os::unix::{fs::PermissionsExt, net::UnixListener};
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("nested")).unwrap();
    let marker = root.path().join("must-not-run");
    let script = root.path().join("custom setup");
    fs::write(&script, format!("touch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o600)).unwrap();
    std::os::unix::fs::symlink(&script, root.path().join("linked-init")).unwrap();
    let _socket = UnixListener::bind(root.path().join("socket")).unwrap();
    let scan = read_environment_directory_with_files(&script, true, root.path(), true).unwrap();
    assert_eq!(scan.path, root.path());
    assert_eq!(scan.children, vec![root.path().join("nested")]);
    assert_eq!(scan.files, vec![script, root.path().join("linked-init")]);
    assert!(!marker.exists());
    let folders = read_environment_directory(root.path(), false, root.path()).unwrap();
    assert_eq!(folders.children, scan.children);
    assert!(folders.files.is_empty());
    assert!(
        read_environment_directory_with_files(
            root.path().join("missing").as_path(),
            false,
            root.path(),
            true
        )
        .is_err()
    );
}

#[test]
fn environment_setup_script_scan_retains_a_bounded_listing() {
    let root = tempfile::tempdir().unwrap();
    for index in 0..ENVIRONMENT_DIRECTORY_LIMIT + 2 {
        fs::write(root.path().join(format!("init-{index:04}")), "").unwrap();
    }
    let result =
        read_environment_directory_with_files(root.path(), false, root.path(), true).unwrap();
    assert_eq!(
        result.children.len() + result.files.len(),
        ENVIRONMENT_DIRECTORY_LIMIT
    );
    assert!(result.notice.is_some());
}
