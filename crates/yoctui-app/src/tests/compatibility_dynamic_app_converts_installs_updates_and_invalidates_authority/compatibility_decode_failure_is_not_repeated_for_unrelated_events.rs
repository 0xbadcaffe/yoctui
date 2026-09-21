use super::*;

#[test]
fn compatibility_decode_failure_is_not_repeated_for_unrelated_events() {
    let authority = compatibility_workspace_authority(1).normalize().unwrap();
    let mut unknown = daemon_compatibility_protocol(&authority);
    unknown.capabilities[0].id = "future.unregistered.capability".into();
    let mut snapshot = compatibility_workspace_daemon_snapshot(&authority);
    snapshot.compatibility = Some(unknown);
    let next_sequence = snapshot.sequence + 1;
    let next_generation = snapshot.generation + 1;

    let mut app = yoctui_model::App::new(16, 4096);
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot);
    assert!(
        app.notification
            .take()
            .unwrap()
            .contains("unknown capability ID")
    );

    client
        .apply_event_to_app(
            &mut app,
            &yoctui_protocol::daemon::SequencedEvent {
                sequence: next_sequence,
                generation: next_generation,
                event: yoctui_protocol::daemon::DaemonEvent::Telemetry(
                    yoctui_protocol::daemon::DaemonTelemetry {
                        uptime_seconds: 1,
                        bitbake: yoctui_protocol::daemon::LifecycleState::Running,
                        connected_clients: 1,
                        active_jobs: 1,
                        pty_sessions: 0,
                        queue_depth: 0,
                        pressure: yoctui_protocol::daemon::DaemonPressureCounters::default(),
                        memory_bytes: None,
                        recovery: yoctui_protocol::daemon::DaemonRecoveryState::CleanStart,
                    },
                ),
            },
        )
        .unwrap();
    assert!(app.notification.is_none());
}
