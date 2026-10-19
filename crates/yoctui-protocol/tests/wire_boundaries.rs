use yoctui_protocol::LineFramer;
use yoctui_protocol::{Envelope, MAX_LINE_BYTES, ProtocolError, VERSION, decode_line, encode_line};

fn envelope(message: String) -> Envelope<String> {
    Envelope {
        protocol_version: VERSION,
        sequence: 1,
        correlation_id: None,
        message,
    }
}

#[test]
fn bridge_encoder_rejects_payloads_that_the_receiver_cannot_accept() {
    assert!(matches!(
        encode_line(&envelope("x".repeat(MAX_LINE_BYTES))),
        Err(ProtocolError::TooLarge)
    ));
}

#[test]
fn bridge_exact_limit_payload_round_trips_with_its_delimiter() {
    let overhead = encode_line(&envelope(String::new())).unwrap().len() - 1;
    let message = envelope("x".repeat(MAX_LINE_BYTES - overhead));
    let encoded = encode_line(&message).unwrap();
    assert_eq!(encoded.len(), MAX_LINE_BYTES + 1);
    assert_eq!(decode_line::<String>(&encoded, None).unwrap(), message);
}

#[test]
fn bridge_unicode_correlation_and_sequence_round_trip() {
    let mut message = envelope("茶\n🙂\0".into());
    message.correlation_id = Some("request-λ".into());
    message.sequence = u64::MAX;
    let encoded = encode_line(&message).unwrap();
    assert_eq!(encoded.last(), Some(&b'\n'));
    assert_eq!(encoded.iter().filter(|byte| **byte == b'\n').count(), 1);
    assert_eq!(
        decode_line::<String>(&encoded, Some(u64::MAX - 1)).unwrap(),
        message
    );
}

#[test]
fn bridge_decoder_accepts_both_bare_json_and_crlf() {
    let message = envelope("ok".into());
    let mut encoded = encode_line(&message).unwrap();
    encoded.pop();
    assert_eq!(decode_line::<String>(&encoded, None).unwrap(), message);
    encoded.extend_from_slice(b"\r\n");
    assert_eq!(decode_line::<String>(&encoded, None).unwrap(), message);
}

#[test]
fn bridge_decoder_reports_invalid_utf8_before_json() {
    assert!(matches!(
        decode_line::<String>(&[0xff], None),
        Err(ProtocolError::Utf8)
    ));
}

#[test]
fn bridge_decoder_rejects_empty_truncated_and_trailing_json() {
    for input in [&b""[..], b"{", b"null", b"[]", b"{}{}"] {
        assert!(matches!(
            decode_line::<String>(input, None),
            Err(ProtocolError::Json(_))
        ));
    }
}

#[test]
fn bridge_decoder_rejects_unsupported_protocol_versions() {
    let mut message = envelope("ok".into());
    for version in [0, VERSION + 1, u32::MAX] {
        message.protocol_version = version;
        assert!(
            matches!(decode_line::<String>(&encode_line(&message).unwrap(), None),
            Err(ProtocolError::Version(actual)) if actual == version)
        );
    }
}

#[test]
fn bridge_decoder_rejects_equal_and_backward_sequences() {
    for sequence in [0, 9, 10] {
        let mut message = envelope("ok".into());
        message.sequence = sequence;
        assert!(
            matches!(decode_line::<String>(&encode_line(&message).unwrap(), Some(10)),
            Err(ProtocolError::Sequence { previous: 10, actual }) if actual == sequence)
        );
    }
}

#[test]
fn bridge_decoder_accepts_zero_as_the_first_sequence() {
    let mut message = envelope("ok".into());
    message.sequence = 0;
    assert_eq!(
        decode_line::<String>(&encode_line(&message).unwrap(), None).unwrap(),
        message
    );
}

#[test]
fn bridge_sequence_overflow_is_rejected_instead_of_wrapping() {
    assert!(matches!(
        decode_line::<String>(
            &encode_line(&envelope("ok".into())).unwrap(),
            Some(u64::MAX)
        ),
        Err(ProtocolError::Sequence {
            previous: u64::MAX,
            actual: 1
        })
    ));
}

#[test]
fn bridge_decoder_rejects_an_oversized_bare_payload() {
    assert!(matches!(
        decode_line::<String>(&vec![b'x'; MAX_LINE_BYTES + 1], None),
        Err(ProtocolError::TooLarge)
    ));
}

#[test]
fn line_framer_preserves_empty_lines_and_partial_tails() {
    let mut framer = LineFramer::default();
    assert_eq!(
        framer.push(b"\n\nlast").unwrap(),
        [Vec::<u8>::new(), Vec::new()]
    );
    assert_eq!(framer.pending_len(), 4);
    assert_eq!(framer.push(b" line\n").unwrap(), [b"last line".to_vec()]);
    assert_eq!(framer.pending_len(), 0);
}

#[test]
fn line_framer_empty_input_does_not_flush_a_partial_line() {
    let mut framer = LineFramer::default();
    assert!(framer.push(b"partial").unwrap().is_empty());
    assert!(framer.push(b"").unwrap().is_empty());
    assert_eq!(framer.pending_len(), 7);
}

#[test]
fn line_framer_accepts_exactly_the_limit_before_newline() {
    let mut framer = LineFramer::default();
    assert!(framer.push(&vec![b'x'; MAX_LINE_BYTES]).unwrap().is_empty());
    assert_eq!(framer.pending_len(), MAX_LINE_BYTES);
    assert_eq!(framer.push(b"\n").unwrap()[0].len(), MAX_LINE_BYTES);
}

#[test]
fn line_framer_clears_oversized_input_and_accepts_a_new_stream() {
    let mut framer = LineFramer::default();
    assert!(matches!(
        framer.push(&vec![b'x'; MAX_LINE_BYTES + 1]),
        Err(ProtocolError::TooLarge)
    ));
    assert_eq!(framer.pending_len(), 0);
    assert_eq!(framer.push(b"next\n").unwrap(), [b"next".to_vec()]);
}

#[test]
fn line_framer_is_identical_at_every_utf8_byte_split() {
    let source = "茶\n🙂\nλ".as_bytes();
    for split in 0..=source.len() {
        let mut framer = LineFramer::default();
        let mut frames = framer.push(&source[..split]).unwrap();
        frames.extend(framer.push(&source[split..]).unwrap());
        assert_eq!(frames, ["茶".as_bytes(), "🙂".as_bytes()]);
        assert_eq!(framer.pending_len(), "λ".len());
    }
}

#[test]
fn line_framer_counts_bytes_instead_of_unicode_characters() {
    let mut framer = LineFramer::default();
    framer.push("茶🙂".as_bytes()).unwrap();
    assert_eq!(framer.pending_len(), 7);
}
