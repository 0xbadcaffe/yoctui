use super::*;

#[test]
fn daemon_listener_wakes_for_pending_output_without_peer_input() {
    let paths = test_paths("pending-output");
    let listener = DaemonListener::bind(&paths).unwrap();
    let (mut server, _peer) = event_pair();
    server
        .queue_event_frame(&encode_frame(&42).unwrap())
        .unwrap();
    let started = Instant::now();
    assert!(
        listener
            .wait_for_activity(&[&server], Duration::from_secs(2))
            .unwrap()
    );
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(server.flush_event_frame().unwrap());
    drop(listener);
    cleanup(&paths);
}
