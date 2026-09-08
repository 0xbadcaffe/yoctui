#![cfg(unix)]
use std::{fs, os::unix::fs::DirBuilderExt, process::Command, time::Duration};
use yoctui_protocol::{
    daemon::*,
    daemon_ipc::{DaemonListener, runtime_paths_for},
};

fn snapshot() -> DaemonSnapshot {
    DaemonSnapshot {
        daemon_instance_id: DaemonInstanceId([1; 16]),
        sequence: 1,
        generation: 1,
        workspace: None,
        project_profile: ProjectProfileSummary::NotLoaded,
        bitbake: BitBakeState {
            lifecycle: LifecycleState::Disconnected,
            version: None,
            diagnostic: None,
            capabilities: vec![],
        },
        compatibility: None,
        jobs: vec![],
        raw_executions: vec![],
        raw_history: vec![],
        pty_sessions: vec![],
        pty_screens: vec![],
        clients: vec![],
        recent_logs: vec![],
        build_events: vec![],
        build_progress: None,
        recovery_warnings: vec![],
    }
}

fn check_build(mode: &'static str, expected_success: bool, expected_requests: usize) {
    let root =
        std::env::temp_dir().join(format!("yoctui-daemon-build-{}-{mode}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
    let paths = runtime_paths_for(root.clone(), unsafe { libc::geteuid() }).unwrap();
    let listener = DaemonListener::bind(&paths).unwrap();
    let server = std::thread::spawn(move || {
        let mut connection = listener.accept(Duration::from_secs(10)).unwrap();
        connection
            .set_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        let mut snapshot = snapshot();
        let mut requests = 0;
        while let Ok(message) = connection.receive::<ClientMessage>() {
            match message {
                ClientMessage::Hello(_) => connection
                    .send(&ServerMessage::Hello(DaemonHello {
                        selected_version: ProtocolVersion::CURRENT,
                        daemon_instance_id: snapshot.daemon_instance_id,
                        boot_id: "fixture-boot".into(),
                        capabilities: vec![Capability::StateSnapshots, Capability::BackgroundJobs],
                        limits: ProtocolLimits {
                            maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                            maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                            maximum_pending_requests: 64,
                            maximum_queue_depth: 256,
                            maximum_terminal_rows: 512,
                            maximum_terminal_columns: 512,
                            maximum_clients: 8,
                            maximum_pty_sessions: 8,
                            maximum_scrollback_lines: 1000,
                            maximum_utility_output_bytes: 65536,
                        },
                    }))
                    .unwrap(),
                ClientMessage::Attach { .. } => {
                    if mode == "changed" && requests > 0 {
                        snapshot.workspace = Some(WorkspaceIdentity {
                            canonical_source: "/other".into(),
                            canonical_build: "/other/build".into(),
                            identity_hash: "other".into(),
                        });
                    }
                    connection
                        .send(&ServerMessage::Attached {
                            replayed_through: snapshot.sequence,
                            snapshot: snapshot.clone(),
                        })
                        .unwrap();
                }
                ClientMessage::Command(request) => {
                    requests += 1;
                    assert!(matches!(request.command, DaemonCommand::StartBuild { .. }));
                    assert_eq!(request.expected_generation, Some(snapshot.generation));
                    if mode == "disconnect" {
                        break;
                    }
                    if mode == "protocol_error" {
                        connection
                            .send(&ServerMessage::Error(ProtocolFailure {
                                request_id: Some(request.request_id),
                                code: ProtocolErrorCode::Internal,
                                message: "fixture protocol error".into(),
                                retryable: true,
                            }))
                            .unwrap();
                        continue;
                    }
                    if matches!(mode, "interleaved" | "event_flood") {
                        if mode == "interleaved" {
                            connection
                                .send(&ServerMessage::Ping {
                                    nonce: 7,
                                    deadline_unix_ms: u64::MAX,
                                })
                                .unwrap();
                        }
                        for sequence in 1..=if mode == "event_flood" { 4096 } else { 3 } {
                            connection
                                .send(&ServerMessage::Event(SequencedEvent {
                                    sequence,
                                    generation: sequence,
                                    event: DaemonEvent::RecoveryWarning {
                                        message: "fixture event".into(),
                                    },
                                }))
                                .unwrap();
                        }
                        if mode == "event_flood" {
                            continue;
                        }
                    }
                    let stale = matches!(mode, "stale_forever" | "changed")
                        || (mode == "stale_once" && requests == 1);
                    let outcome = if stale {
                        snapshot.generation += 1;
                        snapshot.sequence += 1;
                        CommandOutcome::Rejected {
                            code: ProtocolErrorCode::StaleGeneration,
                            message: "fixture telemetry advanced".into(),
                            current_generation: snapshot.generation,
                        }
                    } else if mode == "rejected" {
                        CommandOutcome::Rejected {
                            code: ProtocolErrorCode::Conflict,
                            message: "fixture conflict".into(),
                            current_generation: snapshot.generation,
                        }
                    } else {
                        CommandOutcome::Accepted
                    };
                    let request_id = if mode == "wrong_id" {
                        RequestId(999)
                    } else {
                        request.request_id
                    };
                    connection
                        .send(&ServerMessage::CommandResult(CommandResult {
                            request_id,
                            outcome,
                        }))
                        .unwrap();
                    if mode == "accepted_disconnect" {
                        break;
                    }
                }
                ClientMessage::Detach => {
                    let _ = connection.send(&ServerMessage::Detaching);
                    break;
                }
                ClientMessage::Pong { nonce } => assert_eq!(nonce, 7),
                _ => panic!("unexpected fixture request"),
            }
        }
        requests
    });
    let output = Command::new(env!("CARGO_BIN_EXE_yoctui"))
        .args(["daemon", "build", "fixture-image"])
        .env("XDG_RUNTIME_DIR", &root)
        .output()
        .unwrap();
    let requests = server.join().unwrap();
    fs::remove_dir_all(root).unwrap();
    assert_eq!(
        output.status.success(),
        expected_success,
        "{mode}: {output:?}"
    );
    assert_eq!(requests, expected_requests, "{mode}");
}

#[test]
fn daemon_build_reports_rejection_as_failure() {
    check_build("rejected", false, 1);
}
#[test]
fn daemon_build_refreshes_stale_snapshot_without_duplicate_acceptance() {
    check_build("stale_once", true, 2);
}
#[test]
fn daemon_build_stops_after_bounded_stale_retries() {
    check_build("stale_forever", false, 3);
}
#[test]
fn daemon_build_never_retries_ambiguous_io_or_changed_context() {
    check_build("disconnect", false, 1);
    check_build("changed", false, 1);
    check_build("wrong_id", false, 1);
}
#[test]
fn daemon_build_acceptance_survives_detach_disconnect() {
    check_build("accepted_disconnect", true, 1);
}

#[test]
fn daemon_build_services_interleaved_events_and_pings_with_bounded_wait() {
    check_build("interleaved", true, 1);
    check_build("event_flood", false, 1);
    check_build("protocol_error", false, 1);
}
