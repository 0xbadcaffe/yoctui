use super::*;

#[test]
fn raw_pty_rejects_cross_route_stale_tampered_duplicate_and_tracks_cancel_and_loss() {
    let fixture = Fixture::new("pty-denial", "#!/bin/sh\nexit 99\n");
    let authority = authority_for(&fixture, RawInteractionMode::InteractivePty);
    let wire = pty_request(&authority, "raw-request:daemon-pty-denial");
    let mut raw = DaemonRawSupervisor::default();
    raw.replace_compatibility(Some(authority)).unwrap();

    assert!(matches!(
        raw.start(wire.clone()),
        Err(DaemonRawError::Planner(
            RawJobPlannerError::InteractiveRequest
        ))
    ));
    let mut tampered = wire.clone();
    let replacement = if tampered.preview_digest.starts_with("00") {
        "ff"
    } else {
        "00"
    };
    tampered.preview_digest.replace_range(0..2, replacement);
    assert!(raw.prepare_pty(tampered).is_err());
    let mut stale = wire.clone();
    stale.capability_generation += 1;
    assert!(raw.prepare_pty(stale).is_err());

    let start = raw.prepare_pty(wire.clone()).unwrap();
    let other = raw
        .prepare_pty(pty_request(
            &raw.compatibility.clone().unwrap(),
            "raw-request:daemon-pty-other",
        ))
        .unwrap();
    let mut mismatched = start.clone();
    mismatched.command = other.command;
    assert!(matches!(
        raw.activate_pty(&mismatched),
        Err(DaemonRawError::PtyIdentityMismatch)
    ));
    raw.activate_pty(&start).unwrap();
    assert!(matches!(
        raw.activate_pty(&start),
        Err(DaemonRawError::DuplicateRequest(_))
    ));
    raw.pty_started(start.pty_id).unwrap();
    let DaemonRawCancel::Pty { pty_id, state } = raw.cancel_pty(start.pty_id).unwrap().unwrap()
    else {
        panic!("expected Raw PTY cancellation target");
    };
    assert_eq!(pty_id, start.pty_id);
    assert_eq!(state.phase, yoctui_model::RawExecutionPhase::Cancelling);
    let cancelled = raw.pty_finished(start.pty_id, None, None).unwrap().unwrap();
    assert_eq!(
        cancelled.phase,
        yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled)
    );
    let snapshot = DaemonSnapshot {
        daemon_instance_id: yoctui_protocol::daemon::DaemonInstanceId([7; 16]),
        sequence: 0,
        generation: 0,
        workspace: None,
        project_profile: yoctui_protocol::daemon::ProjectProfileSummary::NotLoaded,
        bitbake: yoctui_protocol::daemon::BitBakeState {
            lifecycle: LifecycleState::Disconnected,
            version: None,
            capabilities: Vec::new(),
            diagnostic: None,
        },
        compatibility: None,
        jobs: Vec::new(),
        raw_executions: vec![yoctui_app::raw_execution_snapshot_to_protocol(&cancelled).unwrap()],
        raw_history: Vec::new(),
        pty_sessions: Vec::new(),
        pty_screens: Vec::new(),
        clients: Vec::new(),
        recent_logs: Vec::new(),
        build_events: Vec::new(),
        build_progress: None,
        recovery_warnings: Vec::new(),
    };
    let mut recovered = DaemonRawSupervisor::default();
    recovered.restore_snapshot(&snapshot).unwrap();
    recovered
        .replace_compatibility(raw.compatibility.clone())
        .unwrap();
    let recovered_start = recovered
        .prepare_pty(pty_request(
            &recovered.compatibility.clone().unwrap(),
            "raw-request:daemon-pty-recovered",
        ))
        .unwrap();
    assert_eq!(recovered_start.pty_id.0, RAW_PTY_NAMESPACE | 2);

    let lost_wire = pty_request(
        &raw.compatibility.clone().unwrap(),
        "raw-request:daemon-pty-lost",
    );
    let lost_start = raw.prepare_pty(lost_wire).unwrap();
    raw.activate_pty(&lost_start).unwrap();
    let lost = raw
        .pty_finished(lost_start.pty_id, None, Some("daemon restarted".into()))
        .unwrap()
        .unwrap();
    assert_eq!(
        lost.phase,
        yoctui_model::RawExecutionPhase::Terminal(RawExecutionOutcome::Lost)
    );
}
