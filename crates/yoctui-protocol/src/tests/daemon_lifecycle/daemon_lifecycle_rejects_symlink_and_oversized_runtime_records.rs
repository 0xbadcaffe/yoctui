use super::*;

#[test]
fn daemon_lifecycle_rejects_symlink_and_oversized_runtime_records() {
    let paths = paths("unsafe");
    let listener = DaemonListener::bind(&paths).unwrap();
    let path = runtime_record_path(&paths);
    std::os::unix::fs::symlink("/tmp", &path).unwrap();
    assert!(matches!(
        read_runtime_record(&paths),
        Err(LifecycleRecordError::Unsafe { .. })
    ));
    fs::remove_file(&path).unwrap();
    fs::write(&path, vec![b'x'; MAX_RUNTIME_RECORD_BYTES as usize + 1]).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(SOCKET_MODE)).unwrap();
    assert!(matches!(
        read_runtime_record(&paths),
        Err(LifecycleRecordError::TooLarge)
    ));
    fs::remove_file(path).unwrap();
    drop(listener);
    fs::remove_dir_all(paths.directory.parent().unwrap()).unwrap();
}
