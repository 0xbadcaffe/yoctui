use super::*;

#[test]
fn raw_job_recovery_sanitizes_output_and_maps_running_work_to_lost_once() {
    let mut prior = snapshot();
    prior.raw_executions = vec![raw_job_snapshot()];
    let persisted = DaemonPersistedState::capture(
        &prior,
        99,
        "same-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    assert!(persisted.raw_executions[0].stdout.chunks.is_empty());
    assert_eq!(persisted.raw_executions[0].stdout.retained_bytes, 0);
    assert!(persisted.raw_executions[0].stdout.dropped_bytes > 0);

    let (recovered, report) = recover_persisted_snapshot(snapshot(), &persisted, "same-boot");
    let execution = &recovered.raw_executions[0];
    assert_eq!(
        execution.phase,
        RawExecutionPhaseData::Terminal(RawExecutionOutcomeData::Lost)
    );
    assert_eq!(execution.attachment, RawAttachmentData::Detached);
    assert_eq!(
        execution.result.as_ref().unwrap().outcome,
        RawExecutionOutcomeData::Lost
    );
    execution.validate().unwrap();
    assert_eq!(report.lost_raw_executions, 1);
    assert_eq!(recovered.raw_history.len(), 1);
    assert_eq!(
        recovered.raw_history[0].outcome,
        RawExecutionOutcomeData::Lost
    );
    assert_eq!(
        recovered.raw_history[0].request_id,
        execution.request.request_id
    );

    let persisted_again = DaemonPersistedState::capture(
        &recovered,
        100,
        "same-boot".into(),
        Vec::new(),
        PersistedPreferences::default(),
    );
    let (recovered_again, report_again) =
        recover_persisted_snapshot(snapshot(), &persisted_again, "same-boot");
    assert_eq!(report_again.lost_raw_executions, 0);
    assert_eq!(
        recovered_again.raw_executions[0],
        recovered.raw_executions[0]
    );
    assert_eq!(recovered_again.raw_history, recovered.raw_history);
}
