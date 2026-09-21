use super::*;

#[test]
fn initial_daemon_attach_retains_authority_after_delayed_snapshot() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-initial-attach-{}-{}",
        std::process::id(),
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir(&root).unwrap();
    fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
    let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
    let listener = DaemonListener::bind(&paths).unwrap();
    let instance = DaemonInstanceId([11; 16]);
    let server = thread::spawn(move || {
        let mut connection = listener.accept(Duration::from_secs(5)).unwrap();
        connection
            .set_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Hello(_)
        ));
        connection
            .send(&ServerMessage::Hello(hello(instance)))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Attach { .. }
        ));
        // Larger live inventories can take longer than the former startup budget.
        thread::sleep(Duration::from_millis(400));
        let mut state = snapshot(instance);
        state.workspace = Some(yoctui_protocol::daemon::WorkspaceIdentity {
            canonical_source: "/live/openbmc".into(),
            canonical_build: "/live/build-romulus".into(),
            identity_hash: "romulus-fixture".into(),
        });
        let _ = connection.send(&ServerMessage::Attached {
            snapshot: state,
            replayed_through: 0,
        });
    });
    let mut client = DaemonClientTransport::connect_at(
        &paths,
        ClientId([12; 16]),
        "initial-attach-test".into(),
        crate::client_runtime::INITIAL_DAEMON_ATTACH_TIMEOUT,
    )
    .unwrap();
    let result = client.attach(
        None,
        Subscription {
            state: true,
            jobs: true,
            logs: true,
            pty_sessions: vec![],
        },
        None,
    );
    server.join().unwrap();
    drop(client);
    fs::remove_dir_all(root).unwrap();
    let attached =
        result.expect("healthy delayed snapshot must retain daemon authority at startup");
    let mut app = yoctui_model::App::new(10, 100);
    let mut replica = yoctui_app::DaemonClientSnapshot::default();
    replica.begin_synchronization();
    replica.replace_app(&mut app, attached.snapshot);
    assert_eq!(
        app.workspace.build_dir.as_deref(),
        Some(std::path::Path::new("/live/build-romulus"))
    );
}
