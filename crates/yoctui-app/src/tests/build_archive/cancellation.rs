use super::*;

#[test]
fn archive_capture_preserves_explicit_daemon_cancellation_after_reconnect() {
    let mut state = snapshot();
    state.jobs[0].lifecycle = LifecycleState::Cancelled;
    state.jobs[0].exit_code = Some(1);
    state
        .build_events
        .retain(|event| !matches!(event, Event::Completed { .. }));
    state.build_events.push(Event::Cancelled {
        exit_code: Some(1),
        finished_unix_ms: Some(500),
    });
    let captured = capture_saved_build(&state, 1000).unwrap();
    assert_eq!(captured.outcome, SavedBuildOutcome::Cancelled);
    assert_eq!(captured.finished_unix_ms, Some(500));
    let decoded = decode_frame::<DaemonSnapshot>(&encode_frame(&state).unwrap()).unwrap();
    assert_eq!(
        capture_saved_build(&decoded, 1000).unwrap().outcome,
        SavedBuildOutcome::Cancelled
    );
}
