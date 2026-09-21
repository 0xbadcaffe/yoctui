use super::*;

#[test]
fn raw_execution_terminal_failure_and_loss_paths_validate() {
    for (outcome, exit_code) in [
        (RawExecutionOutcome::Failed, Some(2)),
        (RawExecutionOutcome::Lost, None),
    ] {
        let mut state = queued(RawInteractionMode::NoninteractiveJob);
        apply(
            &mut state,
            RawExecutionEventKind::Finished {
                result: RawExecutionResult {
                    outcome,
                    exit_code,
                    message: Some("terminal".into()),
                    elapsed_ms: 1,
                    durable_reference: None,
                },
            },
        );
        assert_eq!(state.phase, RawExecutionPhase::Terminal(outcome));
    }
}
