use super::*;

#[test]
fn daemon_ipc_sends_one_preencoded_frame_without_reserialization() {
    let paths = test_paths("preencoded-round-trip");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let mut client = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        client.set_timeout(Some(Duration::from_secs(1))).unwrap();
        client.receive::<ClientMessage>().unwrap()
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server.set_timeout(Some(Duration::from_secs(1))).unwrap();
    let frame = encode_frame(&ClientMessage::Pong { nonce: 23 }).unwrap();
    server.send_encoded_frame(&frame).unwrap();
    assert_eq!(client.join().unwrap(), ClientMessage::Pong { nonce: 23 });
    assert!(matches!(
        server.send_encoded_frame(&[0, 0, 0, 2, b'{']),
        Err(IpcError::Protocol(DaemonProtocolError::InvalidLength))
    ));
    drop(listener);
    cleanup(&paths);
}
