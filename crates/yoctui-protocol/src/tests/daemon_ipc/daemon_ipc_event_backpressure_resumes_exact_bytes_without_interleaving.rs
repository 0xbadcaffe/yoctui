use super::*;

#[test]
fn daemon_ipc_event_backpressure_resumes_exact_bytes_without_interleaving() {
    let paths = test_paths("event-backpressure");
    let listener = DaemonListener::bind(&paths).unwrap();
    let (mut server, mut peer) = event_pair();
    let frame = encode_frame(&"x".repeat(1024 * 1024)).unwrap();
    server.queue_event_frame(&frame).unwrap();
    // Deliberately leave the peer unread until its socket is full.
    for _ in 0..32 {
        assert!(!server.flush_event_frame().unwrap());
    }
    assert!(server.event_write_pending());
    server.pending = encode_frame(&ClientMessage::Pong { nonce: 3 }).unwrap();
    assert!(
        !listener
            .wait_for_activity(&[&server], Duration::from_millis(10))
            .unwrap()
    );
    assert!(server.send(&ClientMessage::Pong { nonce: 1 }).is_err());
    assert!(server.queue_event_frame(&frame).is_err());
    let expected = frame.clone();
    let reader = thread::spawn(move || {
        let mut received = vec![0; expected.len()];
        peer.read_exact(&mut received).unwrap();
        assert_eq!(received, expected);
        let next = encode_frame(&ClientMessage::Pong { nonce: 2 }).unwrap();
        let mut received = vec![0; next.len()];
        peer.read_exact(&mut received).unwrap();
        assert_eq!(received, next);
    });
    while !server.flush_event_frame().unwrap() {
        thread::yield_now();
    }
    server.send(&ClientMessage::Pong { nonce: 2 }).unwrap();
    reader.join().unwrap();
    drop(listener);
    cleanup(&paths);
}
