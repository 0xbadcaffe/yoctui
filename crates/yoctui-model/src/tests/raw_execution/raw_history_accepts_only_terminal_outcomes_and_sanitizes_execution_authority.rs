use super::*;

#[test]
fn raw_history_accepts_only_terminal_outcomes_and_sanitizes_execution_authority() {
    assert_eq!(
        RawHistoryRecord::from_terminal(&queued(RawInteractionMode::NoninteractiveJob)),
        Err(RawExecutionError::HistoryRequiresTerminal)
    );
    for outcome in [
        RawExecutionOutcome::Succeeded,
        RawExecutionOutcome::Failed,
        RawExecutionOutcome::Cancelled,
        RawExecutionOutcome::Lost,
    ] {
        let state = terminal_for_history(&format!("{outcome:?}"), outcome, 150);
        let record = RawHistoryRecord::from_terminal(&state).unwrap();
        assert_eq!(record.outcome, outcome);
        assert_eq!(record.parameters, state.request.parameters);
        assert_eq!(record.started_unix_ms, state.started_unix_ms.unwrap_or(100));
        assert_eq!(record.ended_unix_ms, 150);
    }
    let mut sensitive = terminal_for_history("sensitive", RawExecutionOutcome::Succeeded, 150);
    sensitive.request.parameters.insert(
        RawParameterId::new("free-text").unwrap(),
        RawParameterValue::Text("token-like-value".into()),
    );
    sensitive.request.parameters.insert(
        RawParameterId::new("temporary-file").unwrap(),
        RawParameterValue::File("/tmp/private-input".into()),
    );
    let sanitized = RawHistoryRecord::from_terminal(&sensitive).unwrap();
    assert!(
        !sanitized
            .parameters
            .contains_key(&RawParameterId::new("free-text").unwrap())
    );
    assert!(
        !sanitized
            .parameters
            .contains_key(&RawParameterId::new("temporary-file").unwrap())
    );
}
