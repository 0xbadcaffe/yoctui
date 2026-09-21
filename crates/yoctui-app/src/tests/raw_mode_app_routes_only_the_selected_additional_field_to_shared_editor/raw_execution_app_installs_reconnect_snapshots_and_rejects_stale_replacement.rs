use super::*;

#[test]
fn raw_execution_app_installs_reconnect_snapshots_and_rejects_stale_replacement() {
    let mut state = raw_execution_state_fixture();
    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Starting {
            owner: yoctui_model::RawExecutionOwner::Job(
                yoctui_model::RawJobId::new("raw-job:app-1").unwrap(),
            ),
        },
    );
    let global = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([9; 16]),
        10,
        "raw-app".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&global);
    snapshot.raw_executions = vec![raw_execution_snapshot_to_protocol(&state).unwrap()];
    let mut app = yoctui_model::App::new(16, 4_096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(
        app.raw_mode.execution_states.get(&state.request.id),
        Some(&state)
    );

    apply_raw_execution_fixture(
        &mut state,
        yoctui_model::RawExecutionEventKind::Running {
            started_unix_ms: 20,
        },
    );
    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: snapshot.sequence + 1,
                generation: snapshot.generation + 1,
                event: yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                    raw_execution_snapshot_to_protocol(&state).unwrap(),
                )),
            },
        )
        .unwrap();
    assert_eq!(
        app.raw_mode.execution_states[&state.request.id].phase,
        yoctui_model::RawExecutionPhase::Running
    );

    let before = app.raw_mode.execution_states.clone();
    let stale = snapshot.raw_executions[0].clone();
    assert!(
        client
            .apply_event_to_app(
                &mut app,
                &yoctui_protocol::daemon::SequencedEvent {
                    sequence: snapshot.sequence + 2,
                    generation: snapshot.generation + 2,
                    event: yoctui_protocol::daemon::DaemonEvent::RawExecutionChanged(Box::new(
                        stale,
                    )),
                },
            )
            .is_err()
    );
    assert_eq!(app.raw_mode.execution_states, before);

    client.disconnect_app(&mut app);
    assert!(app.raw_mode.execution_states.is_empty());
}
