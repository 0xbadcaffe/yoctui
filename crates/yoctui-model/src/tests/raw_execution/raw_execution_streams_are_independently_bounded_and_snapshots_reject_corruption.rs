use super::*;

#[test]
fn raw_execution_streams_are_independently_bounded_and_snapshots_reject_corruption() {
    let mut output = RawRetainedOutput::new(
        RawStreamId::new("raw-stream:bounded").unwrap(),
        RawOutputStream::Stdout,
    );
    for sequence in 1..=20 {
        output
            .append(RawOutputChunk {
                stream_id: output.stream_id.clone(),
                stream: RawOutputStream::Stdout,
                sequence,
                text: "界".repeat(MAX_RAW_OUTPUT_CHUNK_BYTES / 3),
                truncated_bytes: 0,
                dropped_lines: 0,
            })
            .unwrap();
    }
    assert!(output.retained_bytes <= MAX_RAW_OUTPUT_RETAINED_BYTES);
    assert!(output.retained_lines <= MAX_RAW_OUTPUT_RETAINED_LINES);
    assert!(output.dropped_bytes > 0);
    assert!(output.validate().is_ok());

    let mut corrupt = queued(RawInteractionMode::NoninteractiveJob);
    corrupt.stdout.retained_bytes = 1;
    let mut installed = Some(queued(RawInteractionMode::NoninteractiveJob));
    let before = installed.clone();
    assert_eq!(
        replace_raw_execution_snapshot(&mut installed, corrupt),
        Err(RawExecutionError::InvalidOutputSnapshot)
    );
    assert_eq!(installed, before);

    let oversized = RawOutputChunk {
        stream_id: RawStreamId::new("raw-stream:oversized").unwrap(),
        stream: RawOutputStream::Stderr,
        sequence: 1,
        text: "é".repeat(MAX_RAW_OUTPUT_CHUNK_BYTES / 2 + 1),
        truncated_bytes: 0,
        dropped_lines: 0,
    };
    assert_eq!(
        oversized.validate(),
        Err(RawExecutionError::InvalidOutputChunk)
    );
}
