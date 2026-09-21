use super::*;

#[test]
fn daemon_ipc_binds_private_socket_and_authenticates_peer() {
    let paths = test_paths("round-trip");
    let listener = DaemonListener::bind(&paths).unwrap();
    let metadata = fs::symlink_metadata(listener.socket_path()).unwrap();
    assert!(metadata.file_type().is_socket());
    assert_eq!(metadata.permissions().mode() & 0o777, SOCKET_MODE);
    assert_eq!(metadata.uid(), effective_uid());

    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let mut client = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        client.set_timeout(Some(Duration::from_secs(1))).unwrap();
        client.send(&ClientMessage::Pong { nonce: 19 }).unwrap();
        client.peer_uid().unwrap()
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server.set_timeout(Some(Duration::from_secs(1))).unwrap();
    assert_eq!(
        server.receive::<ClientMessage>().unwrap(),
        ClientMessage::Pong { nonce: 19 }
    );
    assert_eq!(client.join().unwrap(), effective_uid());
    drop(listener);
    assert!(!paths.socket.exists());
    cleanup(&paths);
}
