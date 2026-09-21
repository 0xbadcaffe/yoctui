use super::*;

#[test]
fn raw_pty_execution_cancellation_and_fail_closed_correlation_are_exact() {
    let mut state = queued(RawInteractionMode::InteractivePty);
    apply(
        &mut state,
        RawExecutionEventKind::Starting {
            owner: RawExecutionOwner::Pty(RawSessionId::new("raw-session:interactive-1").unwrap()),
        },
    );
    apply(
        &mut state,
        RawExecutionEventKind::Running {
            started_unix_ms: 101,
        },
    );
    apply(&mut state, RawExecutionEventKind::CancellationRequested);
    apply(&mut state, RawExecutionEventKind::Cancelling);
    let before = state.clone();
    let stale = RawExecutionEvent {
        request_id: state.request.id.clone(),
        sequence: state.cursor.sequence,
        generation: state.cursor.generation,
        kind: RawExecutionEventKind::Elapsed { elapsed_ms: 999 },
    };
    assert!(!reduce_raw_execution(&mut state, stale).unwrap());
    assert_eq!(state, before);

    let gap = RawExecutionEvent {
        request_id: state.request.id.clone(),
        sequence: state.cursor.sequence + 2,
        generation: state.cursor.generation + 2,
        kind: RawExecutionEventKind::Elapsed { elapsed_ms: 999 },
    };
    assert!(matches!(
        reduce_raw_execution(&mut state, gap),
        Err(RawExecutionError::EventGap { .. })
    ));
    assert_eq!(state, before);

    apply(
        &mut state,
        RawExecutionEventKind::Finished {
            result: RawExecutionResult {
                outcome: RawExecutionOutcome::Cancelled,
                exit_code: None,
                message: None,
                elapsed_ms: 50,
                durable_reference: None,
            },
        },
    );
    assert_eq!(
        state.phase,
        RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
    );
}
