use super::*;

#[cfg(unix)]
#[test]
fn environment_setup_directory_symlinks_resolve_without_execution() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("real")).unwrap();
    symlink(root.path().join("real"), root.path().join("link")).unwrap();
    symlink(root.path().join("loop"), root.path().join("loop")).unwrap();
    let result = read_environment_directory(&root.path().join("link"), false, root.path()).unwrap();
    assert!(result.path.ends_with("real"));
    assert!(read_environment_directory(&root.path().join("loop"), false, root.path()).is_err());
}
