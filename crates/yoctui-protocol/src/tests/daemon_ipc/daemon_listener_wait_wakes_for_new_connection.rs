use super::*;

#[test]
fn daemon_listener_wait_wakes_for_new_connection() {
    let paths = test_paths("activity-listener");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        thread::sleep(Duration::from_millis(20));
        DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
    });
    let started = Instant::now();

    assert!(
        listener
            .wait_for_activity(&[], Duration::from_secs(1))
            .unwrap()
    );
    assert!(started.elapsed() < Duration::from_millis(500));
    let _server = listener.accept(Duration::ZERO).unwrap();

    drop(client.join().unwrap());
    drop(listener);
    cleanup(&paths);
}
