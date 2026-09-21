use super::*;

#[test]
fn listener_readiness_wakes_promptly_before_a_long_deadline() {
    let paths = test_paths("listener-readiness");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        thread::sleep(Duration::from_millis(30));
        DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
    });

    let started = Instant::now();
    let _server = listener.accept(Duration::from_secs(2)).unwrap();
    assert!(started.elapsed() < Duration::from_millis(250));

    let _client = client.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
