use super::*;

#[test]
fn incremental_send_restores_the_snapshot_write_deadline() {
    let (stream, mut peer) = UnixStream::pair().unwrap();
    let mut connection = DaemonConnection {
        stream,
        server_mode: true,
        pending: Vec::new(),
        expected_frame_len: None,
        write_poisoned: false,
        outgoing: None,
    };
    let snapshot_deadline = Duration::from_secs(1);
    connection
        .set_write_timeout(Some(snapshot_deadline))
        .unwrap();
    let frame = encode_frame(&ClientMessage::Pong { nonce: 37 }).unwrap();

    connection
        .send_encoded_frame_with_timeout(&frame, Duration::from_millis(2))
        .unwrap();

    assert_eq!(
        connection.stream.write_timeout().unwrap(),
        Some(snapshot_deadline)
    );
    let mut received = vec![0; frame.len()];
    peer.read_exact(&mut received).unwrap();
    assert_eq!(received, frame);
}
