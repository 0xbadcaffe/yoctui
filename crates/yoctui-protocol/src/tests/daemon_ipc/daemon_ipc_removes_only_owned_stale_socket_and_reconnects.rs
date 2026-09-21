use super::*;

#[test]
fn daemon_ipc_removes_only_owned_stale_socket_and_reconnects() {
    let paths = test_paths("stale");
    fs::create_dir(&paths.directory).unwrap();
    fs::set_permissions(
        &paths.directory,
        fs::Permissions::from_mode(RUNTIME_DIRECTORY_MODE),
    )
    .unwrap();
    let stale = UnixListener::bind(&paths.socket).unwrap();
    drop(stale);
    let listener = DaemonListener::bind(&paths).unwrap();

    let (started_tx, started_rx) = mpsc::channel();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        started_tx.send(()).unwrap();
        DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
    });
    started_rx.recv().unwrap();
    let _server = listener.accept(Duration::from_secs(1)).unwrap();
    let _client = client.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
