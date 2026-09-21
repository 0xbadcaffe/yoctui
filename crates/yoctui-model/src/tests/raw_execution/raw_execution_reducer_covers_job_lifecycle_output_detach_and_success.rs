use super::*;

#[test]
fn raw_execution_reducer_covers_job_lifecycle_output_detach_and_success() {
    let mut state = queued(RawInteractionMode::NoninteractiveJob);
    apply(
        &mut state,
        RawExecutionEventKind::Starting {
            owner: RawExecutionOwner::Job(RawJobId::new("raw-job:one").unwrap()),
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::Running {
            started_unix_ms: 125,
        },
    );
    let stdout = state.stdout.stream_id.clone();
    apply(
        &mut state,
        RawExecutionEventKind::Output {
            chunk: RawOutputChunk {
                stream_id: stdout,
                stream: RawOutputStream::Stdout,
                sequence: 1,
                text: "héllo\n".into(),
                truncated_bytes: 3,
                dropped_lines: 1,
            },
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::AttachmentChanged {
            attachment: RawAttachmentState::Detached,
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::AttachmentChanged {
            attachment: RawAttachmentState::Attached,
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::AttachmentChanged {
            attachment: RawAttachmentState::Detached,
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::Elapsed { elapsed_ms: 80 },
    );
    apply(
        &mut state,
        RawExecutionEventKind::Finished {
            result: RawExecutionResult {
                outcome: RawExecutionOutcome::Succeeded,
                exit_code: Some(0),
                message: Some("complete".into()),
                elapsed_ms: 90,
                durable_reference: Some(
                    RawDurableReferenceId::new("raw-durable:history-1").unwrap(),
                ),
            },
        },
    );
    assert_eq!(
        state.phase,
        RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded)
    );
    assert_eq!(state.attachment, RawAttachmentState::Detached);
    assert_eq!(state.stdout.retained_bytes, "héllo\n".len());
    assert_eq!(state.stdout.dropped_bytes, 3);
    assert_eq!(state.stdout.dropped_lines, 1);
    assert_eq!(state.elapsed_ms, 90);
    assert!(state.validate().is_ok());
}
