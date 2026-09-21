use super::*;

#[test]
fn initial_daemon_attach_missing_socket_fails_without_waiting_for_budget() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-initial-absent-{}-{}",
        std::process::id(),
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
    let started = std::time::Instant::now();
    let result = DaemonClientTransport::connect_at(
        &paths,
        ClientId([13; 16]),
        "absent-attach-test".into(),
        crate::client_runtime::INITIAL_DAEMON_ATTACH_TIMEOUT,
    );
    let elapsed = started.elapsed();
    fs::remove_dir_all(root).unwrap();
    assert!(result.is_err());
    assert!(
        elapsed < Duration::from_secs(1),
        "missing socket waited {elapsed:?}"
    );
    assert_eq!(
        crate::client_runtime::INITIAL_DAEMON_ATTACH_TIMEOUT,
        Duration::from_secs(5)
    );
}
