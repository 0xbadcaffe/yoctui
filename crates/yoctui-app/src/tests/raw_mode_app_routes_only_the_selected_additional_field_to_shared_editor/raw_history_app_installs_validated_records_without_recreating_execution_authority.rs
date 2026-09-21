use super::*;

#[test]
fn raw_history_app_installs_validated_records_without_recreating_execution_authority() {
    let mut state = raw_execution_state_fixture();
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:history-app").unwrap(),
            ),
        },
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Finished {
            result: yoctui_model::RawExecutionResult {
                outcome: yoctui_model::RawExecutionOutcome::Succeeded,
                exit_code: Some(0),
                message: Some("not history".into()),
                elapsed_ms: 50,
                durable_reference: None,
            },
        },
    );
    let execution = raw_execution_snapshot_to_protocol(&state).unwrap();
    let wire = yoctui_protocol::daemon::RawHistoryRecordData::from_terminal(&execution).unwrap();
    let model = raw_history_record_from_protocol(&wire).unwrap();
    assert_eq!(model.command, state.request.command);

    let global = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        10,
        "raw-history-app".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&global);
    snapshot.raw_history = vec![wire];
    let mut app = yoctui_model::App::new(16, 4_096);
    DaemonClientSnapshot::default().replace_app(&mut app, snapshot);
    assert_eq!(app.raw_mode.history, [model]);
    assert!(app.raw_mode.execution_states.is_empty());
}
