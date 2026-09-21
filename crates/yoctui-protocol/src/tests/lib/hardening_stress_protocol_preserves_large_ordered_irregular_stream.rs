use super::*;

#[test]
fn hardening_stress_protocol_preserves_large_ordered_irregular_stream() {
    const FRAMES: u64 = 10_000;
    let mut source = Vec::new();
    for sequence in 1..=FRAMES {
        source.extend(
            encode_line(&Envelope {
                protocol_version: VERSION,
                sequence,
                correlation_id: Some(format!("stress-{sequence}")),
                message: Command::Hello {
                    compatibility: None,
                },
            })
            .unwrap(),
        );
    }

    let mut framer = LineFramer::default();
    let mut decoded = 0_u64;
    let mut offset = 0;
    while offset < source.len() {
        let chunk_size = ((offset * 37 + 11) % 97) + 1;
        let end = (offset + chunk_size).min(source.len());
        for frame in framer.push(&source[offset..end]).unwrap() {
            let envelope = decode_line::<Command>(&frame, Some(decoded)).unwrap();
            decoded += 1;
            assert_eq!(envelope.sequence, decoded);
            assert_eq!(envelope.correlation_id, Some(format!("stress-{decoded}")));
            assert_eq!(
                envelope.message,
                Command::Hello {
                    compatibility: None
                }
            );
        }
        offset = end;
    }
    assert_eq!(decoded, FRAMES);
    assert_eq!(framer.pending_len(), 0);
}
