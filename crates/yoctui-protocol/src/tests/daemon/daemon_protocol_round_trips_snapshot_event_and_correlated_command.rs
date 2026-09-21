use super::*;

#[test]
fn daemon_protocol_round_trips_snapshot_event_and_correlated_command() {
    let snapshot = DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([7; 16]),
        sequence: 42,
        generation: 9,
        workspace: None,
        project_profile: ProjectProfileSummary::Absent,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Running,
            version: Some("2.8.1".into()),
            capabilities: vec![BitBakeCapability::WorkspaceInspection],
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: Vec::new(),
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: vec![ClientSummary {
            id: client_id(1),
            name: "ssh-client".into(),
            attached_unix_ms: 1,
            last_seen_unix_ms: 2,
        }],
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    };
    let message = ServerMessage::Attached {
        snapshot,
        replayed_through: 42,
    };
    assert_eq!(
        decode_frame::<ServerMessage>(&encode_frame(&message).unwrap()).unwrap(),
        message
    );

    let command = ClientMessage::Command(CommandRequest {
        request_id: RequestId(81),
        expected_generation: Some(9),
        command: DaemonCommand::TakePtyControl {
            session_id: PtySessionId(3),
            expected_epoch: 0,
        },
    });
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&command).unwrap()).unwrap(),
        command
    );
}
