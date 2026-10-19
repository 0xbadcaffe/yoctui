use yoctui_protocol::daemon::*;

#[test]
fn frame_length_prefix_is_big_endian_and_excludes_itself() {
    let encoded = encode_frame(&"hello").unwrap();
    assert_eq!(&encoded[..4], &7u32.to_be_bytes());
    assert_eq!(&encoded[4..], b"\"hello\"");
}

#[test]
fn frame_round_trips_unicode_null_and_embedded_newlines() {
    let text = "茶\0🙂\n\r\x1b";
    assert_eq!(
        decode_frame::<String>(&encode_frame(&text).unwrap()).unwrap(),
        text
    );
}

#[test]
fn frame_decoder_rejects_every_short_prefix() {
    for length in 0..4 {
        assert!(matches!(
            decode_frame::<String>(&[0; 4][..length]),
            Err(DaemonProtocolError::InvalidLength)
        ));
    }
}

#[test]
fn frame_decoder_rejects_length_mismatch_in_both_directions() {
    let encoded = encode_frame(&"a").unwrap();
    assert!(matches!(
        decode_frame::<String>(&encoded[..encoded.len() - 1]),
        Err(DaemonProtocolError::InvalidLength)
    ));
    let mut trailing = encoded.clone();
    trailing.push(0);
    assert!(matches!(
        decode_frame::<String>(&trailing),
        Err(DaemonProtocolError::InvalidLength)
    ));
}

#[test]
fn frame_decoder_rejects_zero_length_invalid_json() {
    assert!(matches!(
        decode_frame::<String>(&[0, 0, 0, 0]),
        Err(DaemonProtocolError::Json(_))
    ));
}

#[test]
fn frame_decoder_rejects_invalid_utf8_json() {
    assert!(matches!(
        decode_frame::<String>(&[0, 0, 0, 1, 255]),
        Err(DaemonProtocolError::Json(_))
    ));
}

#[test]
fn frame_decoder_rejects_impossibly_large_advertised_lengths() {
    assert!(matches!(
        decode_frame::<String>(&u32::MAX.to_be_bytes()),
        Err(DaemonProtocolError::TooLarge)
    ));
}

#[test]
fn frame_encoder_and_decoder_agree_at_the_exact_size_limit() {
    let text = "x".repeat(MAX_FRAME_BYTES - 2);
    let encoded = encode_frame(&text).unwrap();
    assert_eq!(encoded.len(), MAX_FRAME_BYTES + 4);
    assert_eq!(decode_frame::<String>(&encoded).unwrap(), text);
}

#[test]
fn frame_encoder_rejects_a_payload_one_byte_over_limit() {
    assert!(matches!(
        encode_frame(&"x".repeat(MAX_FRAME_BYTES - 1)),
        Err(DaemonProtocolError::TooLarge)
    ));
}

#[test]
fn streaming_decoder_accepts_every_prefix_and_unicode_split() {
    let encoded = encode_frame(&"茶🙂").unwrap();
    for split in 0..=encoded.len() {
        let mut decoder = FrameDecoder::default();
        let mut frames = decoder.push(&encoded[..split]).unwrap();
        frames.extend(decoder.push(&encoded[split..]).unwrap());
        assert_eq!(frames.as_slice(), std::slice::from_ref(&encoded));
        assert_eq!(decoder.pending_len(), 0);
    }
}

#[test]
fn streaming_decoder_retains_partial_payload_after_complete_frames() {
    let first = encode_frame(&1).unwrap();
    let second = encode_frame(&2).unwrap();
    let mut combined = first.clone();
    combined.extend_from_slice(&second[..4]);
    let mut decoder = FrameDecoder::default();
    assert_eq!(decoder.push(&combined).unwrap(), [first]);
    assert_eq!(decoder.pending_len(), 4);
    assert_eq!(decoder.push(&second[4..]).unwrap(), [second]);
}

#[test]
fn streaming_decoder_empty_push_does_not_discard_pending_bytes() {
    let mut decoder = FrameDecoder::default();
    decoder.push(&[0, 0]).unwrap();
    assert!(decoder.push(&[]).unwrap().is_empty());
    assert_eq!(decoder.pending_len(), 2);
}

#[test]
fn streaming_decoder_recovers_after_rejecting_an_oversize_header() {
    let mut decoder = FrameDecoder::default();
    assert!(matches!(
        decoder.push(&u32::MAX.to_be_bytes()),
        Err(DaemonProtocolError::TooLarge)
    ));
    assert_eq!(decoder.pending_len(), 0);
    let valid = encode_frame(&"new stream").unwrap();
    assert_eq!(decoder.push(&valid).unwrap(), [valid]);
}

#[test]
fn streaming_decoder_accepts_aggregate_batches_larger_than_one_frame() {
    let frame = encode_frame(&"x".repeat(MAX_FRAME_BYTES / 2)).unwrap();
    let combined = frame.repeat(3);
    let mut decoder = FrameDecoder::default();
    assert_eq!(decoder.push(&combined).unwrap(), vec![frame; 3]);
    assert_eq!(decoder.pending_len(), 0);
}

fn version(major: u16, minor: u16) -> ProtocolVersion {
    ProtocolVersion { major, minor }
}

#[test]
fn version_negotiation_uses_the_highest_shared_minor() {
    assert_eq!(
        negotiate_version(version(1, 2), version(1, 5), version(1, 3)).unwrap(),
        version(1, 3)
    );
    assert_eq!(
        negotiate_version(version(1, 2), version(1, 5), version(1, 9)).unwrap(),
        version(1, 5)
    );
}

#[test]
fn version_negotiation_rejects_inverted_ranges_and_major_mismatches() {
    for (minimum, maximum, daemon) in [
        (version(1, 5), version(1, 2), version(1, 6)),
        (version(1, 2), version(2, 5), version(1, 3)),
        (version(1, 2), version(1, 5), version(2, 3)),
        (version(1, 2), version(1, 5), version(1, 1)),
    ] {
        assert!(matches!(
            negotiate_version(minimum, maximum, daemon),
            Err(DaemonProtocolError::IncompatibleVersion)
        ));
    }
}

#[test]
fn version_negotiation_accepts_an_exact_zero_minor_range() {
    assert_eq!(
        negotiate_version(version(1, 0), version(1, 0), version(1, 0)).unwrap(),
        version(1, 0)
    );
}

#[test]
fn capability_negotiation_with_an_empty_side_is_empty() {
    assert!(negotiate_capabilities(&[], &[]).unwrap().is_empty());
}

#[test]
fn capability_negotiation_rejects_over_limit_lists_even_without_common_values() {
    let oversized = vec![Capability::StateSnapshots; MAX_CAPABILITIES + 1];
    assert!(matches!(
        negotiate_capabilities(&oversized, &[]),
        Err(DaemonProtocolError::TooManyCapabilities)
    ));
    assert!(matches!(
        negotiate_capabilities(&[], &oversized),
        Err(DaemonProtocolError::TooManyCapabilities)
    ));
}

#[test]
fn capability_intersection_is_sorted_unique_and_order_independent() {
    let client = [
        Capability::PtySessions,
        Capability::StateSnapshots,
        Capability::PtySessions,
    ];
    let daemon = [
        Capability::RawExecution,
        Capability::StateSnapshots,
        Capability::PtySessions,
    ];
    let expected = [Capability::StateSnapshots, Capability::PtySessions];
    assert_eq!(negotiate_capabilities(&client, &daemon).unwrap(), expected);
    assert_eq!(negotiate_capabilities(&daemon, &client).unwrap(), expected);
}

#[test]
fn capability_lists_at_the_exact_limit_are_accepted_and_deduplicated() {
    let capabilities = vec![Capability::StateSnapshots; MAX_CAPABILITIES];
    assert_eq!(
        negotiate_capabilities(&capabilities, &capabilities).unwrap(),
        [Capability::StateSnapshots]
    );
}

use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig { cases: 128, failure_persistence: None, ..ProptestConfig::default() })]

    #[test]
    fn arbitrarily_fragmented_daemon_frames_preserve_order_and_bytes(
        values in prop::collection::vec(".{0,40}", 0..20), chunk_size in 1usize..30
    ) {
        let encoded: Vec<_> = values.iter().map(|value| encode_frame(value).unwrap()).collect();
        let wire: Vec<_> = encoded.iter().flatten().copied().collect();
        let mut decoder = FrameDecoder::default();
        let mut actual = Vec::new();
        for chunk in wire.chunks(chunk_size) { actual.extend(decoder.push(chunk).unwrap()); }
        prop_assert_eq!(actual, encoded);
        prop_assert_eq!(decoder.pending_len(), 0);
    }

    #[test]
    fn arbitrary_payload_bytes_produce_a_result_not_a_panic(bytes in prop::collection::vec(any::<u8>(), 0..256)) {
        let _ = decode_frame::<serde_json::Value>(&bytes);
        let mut decoder = FrameDecoder::default();
        let _ = decoder.push(&bytes);
        prop_assert!(decoder.pending_len() <= MAX_FRAME_BYTES + 4);
    }
}
