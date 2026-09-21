use super::*;

#[test]
fn daemon_status_event_updates_client_telemetry_without_mutating_snapshot_shape() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([5; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut client = DaemonClientSnapshot::default();
    client.replace(initial);
    client
        .apply_event(&yoctui_protocol::daemon::SequencedEvent {
            sequence: 1,
            generation: 1,
            event: yoctui_protocol::daemon::DaemonEvent::Telemetry(
                yoctui_protocol::daemon::DaemonTelemetry {
                    uptime_seconds: 7,
                    bitbake: yoctui_protocol::daemon::LifecycleState::Running,
                    connected_clients: 2,
                    active_jobs: 1,
                    pty_sessions: 1,
                    queue_depth: 3,
                    pressure: yoctui_protocol::daemon::DaemonPressureCounters {
                        current_queue_depth: 3,
                        maximum_queue_depth: 9,
                        cosmetic_coalesced: 2,
                        cosmetic_dropped: 4,
                        reliable_waits: 1,
                        forced_resynchronizations: 2,
                        slow_client_disconnects: 1,
                    },
                    memory_bytes: Some(4096),
                    recovery: yoctui_protocol::daemon::DaemonRecoveryState::Recovered,
                },
            ),
        })
        .unwrap();
    let telemetry = client.telemetry.unwrap();
    assert_eq!(telemetry.uptime_seconds, 7);
    assert_eq!(telemetry.pressure.current_queue_depth, 3);
    assert_eq!(telemetry.pressure.maximum_queue_depth, 9);
    assert_eq!(telemetry.pressure.cosmetic_coalesced, 2);
    assert_eq!(telemetry.pressure.cosmetic_dropped, 4);
    assert_eq!(telemetry.pressure.reliable_waits, 1);
    assert_eq!(telemetry.pressure.forced_resynchronizations, 2);
    assert_eq!(telemetry.pressure.slow_client_disconnects, 1);
    assert_eq!(client.snapshot.unwrap().sequence, 1);
}
