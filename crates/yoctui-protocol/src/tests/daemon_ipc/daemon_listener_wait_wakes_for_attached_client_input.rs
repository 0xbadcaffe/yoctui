use super::*;

#[test]
fn daemon_listener_wait_wakes_for_attached_client_input() {
    let paths = test_paths("activity-client");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        let mut connection =
            DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap();
        thread::sleep(Duration::from_millis(20));
        connection.send(&ClientMessage::Pong { nonce: 41 }).unwrap();
    });
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    let started = Instant::now();

    assert!(
        listener
            .wait_for_activity(&[&server], Duration::from_secs(1))
            .unwrap()
    );
    assert!(started.elapsed() < Duration::from_millis(500));
    assert_eq!(
        server.receive::<ClientMessage>().unwrap(),
        ClientMessage::Pong { nonce: 41 }
    );

    client.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
