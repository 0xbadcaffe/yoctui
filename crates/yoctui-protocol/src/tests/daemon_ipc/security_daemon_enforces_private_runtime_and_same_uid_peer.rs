use super::*;

#[test]
fn security_daemon_enforces_private_runtime_and_same_uid_peer() {
    let paths = test_paths("security");
    let listener = DaemonListener::bind(&paths).unwrap();
    let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
    assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
    assert_eq!(metadata.uid(), effective_uid());
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let client = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        client.peer_uid().unwrap()
    });
    let server = listener.accept(Duration::from_secs(1)).unwrap();
    assert_eq!(server.peer_uid().unwrap(), effective_uid());
    assert_eq!(client.join().unwrap(), effective_uid());
    drop(listener);
    cleanup(&paths);
}
