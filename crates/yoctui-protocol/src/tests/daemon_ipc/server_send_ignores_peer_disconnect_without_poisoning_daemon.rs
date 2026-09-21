use super::*;

#[test]
fn server_send_ignores_peer_disconnect_without_poisoning_daemon() {
    let paths = test_paths("peer-disconnect");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let mut connection =
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        connection.send(&ClientMessage::Pong { nonce: 1 }).unwrap();
        connection
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server.receive::<ClientMessage>().unwrap();
    drop(client.join().unwrap());
    assert!(server.send(&ClientMessage::Pong { nonce: 7 }).is_ok());
    cleanup(&paths);
}
