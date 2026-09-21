use super::*;

#[test]
fn daemon_ipc_retains_partial_frame_across_read_timeout() {
    let paths = test_paths("partial-timeout");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let connection = DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        let frame = encode_frame(&ClientMessage::Pong { nonce: 29 }).unwrap();
        (&connection.stream).write_all(&frame[..8]).unwrap();
        thread::sleep(Duration::from_millis(60));
        (&connection.stream).write_all(&frame[8..]).unwrap();
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server
        .set_read_timeout(Some(Duration::from_millis(20)))
        .unwrap();

    assert!(matches!(
        server.receive::<ClientMessage>(),
        Err(IpcError::Timeout(_))
    ));
    client.join().unwrap();
    assert_eq!(
        server.receive::<ClientMessage>().unwrap(),
        ClientMessage::Pong { nonce: 29 }
    );

    drop(listener);
    cleanup(&paths);
}
