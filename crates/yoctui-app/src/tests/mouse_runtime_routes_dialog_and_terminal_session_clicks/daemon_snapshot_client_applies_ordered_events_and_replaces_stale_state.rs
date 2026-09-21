use super::*;

#[test]
fn daemon_snapshot_client_applies_ordered_events_and_replaces_stale_state() {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([4; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let initial = daemon_protocol_snapshot(&state);
    let mut client = DaemonClientSnapshot::default();
    client.begin_synchronization();
    client.replace(initial.clone());
    let event = yoctui_protocol::daemon::SequencedEvent {
        sequence: 1,
        generation: 1,
        event: yoctui_protocol::daemon::DaemonEvent::Log(yoctui_protocol::daemon::LogRecord {
            source: "bitbake".into(),
            severity: yoctui_protocol::daemon::LogSeverity::Info,
            message: "parsing".into(),
            unix_ms: 1,
            recipe: None,
            task: None,
            path: None,
            build: None,
        }),
    };
    client.apply_event(&event).unwrap();
    assert_eq!(client.snapshot.as_ref().unwrap().sequence, 1);
    assert_eq!(client.resume_cursor().unwrap().last_sequence, 1);

    let gap = yoctui_protocol::daemon::SequencedEvent {
        sequence: 3,
        generation: 3,
        event: yoctui_protocol::daemon::DaemonEvent::Unknown,
    };
    assert!(matches!(
        client.apply_event(&gap),
        Err(DaemonClientSyncError::Protocol(
            yoctui_protocol::daemon::DaemonSnapshotError::EventGap { .. }
        ))
    ));
    assert_eq!(client.status, yoctui_model::ClientReplicaStatus::Stale);
    assert!(client.resume_cursor().is_none());

    client.begin_synchronization();
    client.replace(initial.clone());
    assert_eq!(client.snapshot.as_ref(), Some(&initial));
    assert_eq!(client.status, yoctui_model::ClientReplicaStatus::Current);
    client.disconnect();
    assert_eq!(
        client.status,
        yoctui_model::ClientReplicaStatus::Disconnected
    );
    assert_eq!(client.snapshot.as_ref(), Some(&initial));
}
