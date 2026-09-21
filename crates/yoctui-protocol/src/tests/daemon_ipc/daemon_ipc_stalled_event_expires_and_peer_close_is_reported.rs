use super::*;

#[test]
fn daemon_ipc_stalled_event_expires_and_peer_close_is_reported() {
    let (mut server, peer) = event_pair();
    let frame = encode_frame(&ClientMessage::Pong { nonce: 1 }).unwrap();
    server.queue_event_frame(&frame).unwrap();
    server.outgoing.as_mut().unwrap().2 = Instant::now() - Duration::from_secs(6);
    assert!(matches!(
        server.flush_event_frame(),
        Err(IpcError::Timeout(_))
    ));
    assert!(matches!(server.send(&1), Err(IpcError::Timeout(_))));
    drop(peer);
    let (mut server, peer) = event_pair();
    server.queue_event_frame(&frame).unwrap();
    drop(peer);
    assert!(server.flush_event_frame().is_err());
    assert!(server.write_poisoned);
    assert!(server.queue_event_frame(&frame).is_err());
}
