use super::*;

#[test]
fn daemon_ipc_readiness_is_nonblocking_and_observes_peer_input() {
    let (stream, mut peer) = UnixStream::pair().unwrap();
    let connection = DaemonConnection {
        stream,
        server_mode: true,
        pending: Vec::new(),
        expected_frame_len: None,
        write_poisoned: false,
        outgoing: None,
    };
    assert!(!connection.is_readable().unwrap());
    peer.write_all(&encode_frame(&ClientMessage::Pong { nonce: 31 }).unwrap())
        .unwrap();
    assert!(connection.is_readable().unwrap());
}
