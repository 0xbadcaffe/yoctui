use super::*;

#[test]
fn daemon_ipc_read_timeout_and_message_bound_are_typed() {
    let paths = test_paths("limits");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let mut connection =
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        connection
            .stream
            .write_all(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes()))
            .unwrap();
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    server.set_timeout(Some(Duration::from_millis(50))).unwrap();
    assert!(matches!(
        server.receive::<ClientMessage>(),
        Err(IpcError::Protocol(DaemonProtocolError::TooLarge))
    ));
    client.join().unwrap();

    let waiting_paths = paths.clone();
    let waiting_client = thread::spawn(move || {
        DaemonConnection::connect(&waiting_paths, Duration::from_secs(1)).unwrap()
    });
    let mut waiting_server = listener.accept(Duration::from_secs(1)).unwrap();
    waiting_server
        .set_timeout(Some(Duration::from_millis(20)))
        .unwrap();
    assert!(matches!(
        waiting_server.receive::<ClientMessage>(),
        Err(IpcError::Timeout(_))
    ));
    drop(waiting_server);
    let _ = waiting_client.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
