use super::*;

#[test]
fn daemon_persist_round_trips_private_bounded_metadata_without_processes() {
    let root = root("round-trip");
    let paths = persist_paths_for(&root).unwrap();
    let persisted = DaemonPersistedState::capture(
        &snapshot(),
        99,
        "boot".into(),
        vec![PersistedClientLayout {
            client_key: "terminal".into(),
            layout_revision: 2,
            session_names: vec!["build".into()],
        }],
        PersistedPreferences {
            theme: Some("dark".into()),
            prefix_key: Some("Ctrl-b".into()),
            recent_logs_enabled: true,
        },
    );
    write_persisted_state(&paths, &persisted).unwrap();
    assert_eq!(read_persisted_state(&paths).unwrap(), Some(persisted));
    assert_eq!(
        fs::symlink_metadata(&paths.state)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        PRIVATE_FILE_MODE
    );
    let text = fs::read_to_string(&paths.state).unwrap();
    assert!(!text.contains("\"pid\""));
    assert!(!text.contains("process_group"));
    fs::remove_dir_all(root).unwrap();
}
