use super::*;

#[test]
fn daemon_ipc_rejects_unsafe_paths_and_reports_unavailable_timeout() {
    assert!(matches!(
        runtime_paths_for(PathBuf::from("relative"), effective_uid()),
        Err(IpcError::RelativeRuntimePath(_))
    ));
    let paths = test_paths("unsafe");
    fs::create_dir(&paths.directory).unwrap();
    fs::set_permissions(&paths.directory, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        DaemonListener::bind(&paths),
        Err(IpcError::UnsafeRuntimePath { .. })
    ));
    fs::set_permissions(
        &paths.directory,
        fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
    )
    .unwrap();
    fs::write(&paths.socket, b"not a socket").unwrap();
    assert!(matches!(
        DaemonListener::bind(&paths),
        Err(IpcError::UnsafeRuntimePath { .. })
    ));
    fs::remove_file(&paths.socket).unwrap();
    std::os::unix::fs::symlink("/tmp", &paths.socket).unwrap();
    assert!(matches!(
        DaemonListener::bind(&paths),
        Err(IpcError::UnsafeRuntimePath { .. })
    ));
    fs::remove_file(&paths.socket).unwrap();
    assert!(matches!(
        DaemonConnection::connect(&paths, Duration::from_millis(20)),
        Err(IpcError::Unavailable { .. })
    ));
    cleanup(&paths);
}
