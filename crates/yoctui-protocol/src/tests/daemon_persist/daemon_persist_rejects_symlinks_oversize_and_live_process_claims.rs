use super::*;

#[test]
fn daemon_persist_rejects_symlinks_oversize_and_live_process_claims() {
    let root = root("unsafe");
    let paths = persist_paths_for(&root).unwrap();
    std::os::unix::fs::symlink("/tmp", &paths.state).unwrap();
    assert!(matches!(
        read_persisted_state(&paths),
        Err(DaemonPersistError::Unsafe { .. })
    ));
    fs::remove_file(&paths.state).unwrap();
    fs::write(
        &paths.state,
        vec![b'x'; MAX_DAEMON_PERSIST_BYTES as usize + 1],
    )
    .unwrap();
    fs::set_permissions(&paths.state, fs::Permissions::from_mode(PRIVATE_FILE_MODE)).unwrap();
    assert!(matches!(
        read_persisted_state(&paths),
        Err(DaemonPersistError::TooLarge)
    ));
    fs::remove_file(&paths.state).unwrap();

    let mut persisted = DaemonPersistedState::capture(
        &snapshot(),
        99,
        "boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    persisted.terminal_sessions.push(PersistedTerminalSession {
        id: 1,
        name: "unsafe".into(),
        kind: PtyKind::Utility,
        cwd: "/tmp".into(),
        previous_lifecycle: LifecycleState::Running,
        dimensions: TerminalDimensions {
            columns: 80,
            rows: 24,
        },
        exit_code: None,
        restartable: true,
        live_process_persisted: true,
    });
    assert!(matches!(
        write_persisted_state(&paths, &persisted),
        Err(DaemonPersistError::Unsafe { .. })
    ));
    fs::remove_dir_all(root).unwrap();
}
