use super::*;

#[test]
fn daemon_ipc_configures_read_and_write_deadlines_independently() {
    let paths = test_paths("independent-timeouts");
    let listener = DaemonListener::bind(&paths).unwrap();
    let client_paths = paths.clone();
    let client = thread::spawn(move || {
        DaemonConnection::connect(&client_paths, Duration::from_secs(1)).unwrap()
    });
    let server = listener.accept(Duration::from_secs(1)).unwrap();
    let read_timeout = Duration::from_millis(20);
    let write_timeout = Duration::from_secs(2);

    server.set_read_timeout(Some(read_timeout)).unwrap();
    server.set_write_timeout(Some(write_timeout)).unwrap();

    assert_eq!(server.stream.read_timeout().unwrap(), Some(read_timeout));
    assert_eq!(server.stream.write_timeout().unwrap(), Some(write_timeout));
    drop(client.join().unwrap());
    drop(listener);
    cleanup(&paths);
}
