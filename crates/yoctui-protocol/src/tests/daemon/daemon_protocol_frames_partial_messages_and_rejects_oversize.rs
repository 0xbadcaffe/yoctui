use super::*;

#[test]
fn daemon_protocol_frames_partial_messages_and_rejects_oversize() {
    let first = encode_frame(&ClientMessage::Detach).unwrap();
    let second = encode_frame(&ClientMessage::Pong { nonce: 4 }).unwrap();
    let mut decoder = FrameDecoder::default();
    assert!(decoder.push(&first[..3]).unwrap().is_empty());
    let mut remainder = first[3..].to_vec();
    remainder.extend_from_slice(&second);
    let frames = decoder.push(&remainder).unwrap();
    assert_eq!(frames.len(), 2);
    assert_eq!(
        decode_frame::<ClientMessage>(&frames[0]).unwrap(),
        ClientMessage::Detach
    );
    assert_eq!(
        decode_frame::<ClientMessage>(&frames[1]).unwrap(),
        ClientMessage::Pong { nonce: 4 }
    );
    assert_eq!(decoder.pending_len(), 0);

    let mut oversized = FrameDecoder::default();
    assert!(matches!(
        oversized.push(&((MAX_FRAME_BYTES as u32 + 1).to_be_bytes())),
        Err(DaemonProtocolError::TooLarge)
    ));
}
