use super::*;

#[test]
fn server_write_timeout_poisoning_prevents_a_followup_frame() {
    let paths = test_paths("write-timeout");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let connection = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        thread::sleep(Duration::from_millis(200));
        connection
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server
        .set_write_timeout(Some(Duration::from_millis(20)))
        .unwrap();

    server.send(&"x".repeat(3 * 1024 * 1024)).unwrap();
    assert!(server.write_poisoned);
    assert!(matches!(
        server.send(&ClientMessage::Pong { nonce: 31 }),
        Err(IpcError::Disconnected)
    ));

    drop(client.join().unwrap());
    drop(listener);
    cleanup(&paths);
}
