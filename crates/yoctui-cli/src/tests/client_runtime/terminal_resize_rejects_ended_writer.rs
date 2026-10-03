use super::*;
use std::{fs, os::unix::fs::PermissionsExt, thread};
use yoctui_protocol::{
    daemon::{
        BitBakeState, Capability, ClientId, ClientMessage, DaemonHello, DaemonInstanceId,
        DaemonSnapshot, LifecycleState, MAX_FRAME_BYTES, ProjectProfileSummary, ProtocolLimits,
        ProtocolVersion, ServerMessage, Subscription,
    },
    daemon_ipc::{DaemonListener, runtime_paths_for},
};

#[test]
fn terminal_resize_rejects_ended_writer_without_sending_a_request() {
    let root = tempfile::tempdir().unwrap();
    fs::set_permissions(root.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let paths = runtime_paths_for(root.path().to_path_buf(), unsafe { libc::geteuid() }).unwrap();
    let listener = DaemonListener::bind(&paths).unwrap();
    let instance = DaemonInstanceId([9; 16]);
    let server = thread::spawn(move || {
        let mut connection = listener.accept(Duration::from_secs(5)).unwrap();
        connection
            .set_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Hello(_)
        ));
        connection
            .send(&ServerMessage::Hello(DaemonHello {
                selected_version: ProtocolVersion::CURRENT,
                daemon_instance_id: instance,
                boot_id: "terminal-test".into(),
                capabilities: vec![
                    Capability::StateSnapshots,
                    Capability::IncrementalEvents,
                    Capability::GracefulShutdown,
                ],
                limits: ProtocolLimits {
                    maximum_frame_bytes: MAX_FRAME_BYTES as u32,
                    maximum_snapshot_bytes: MAX_FRAME_BYTES as u32,
                    maximum_pending_requests: 8,
                    maximum_queue_depth: 16,
                    maximum_terminal_rows: 512,
                    maximum_terminal_columns: 512,
                    maximum_clients: 32,
                    maximum_pty_sessions: 64,
                    maximum_scrollback_lines: 100_000,
                    maximum_utility_output_bytes: 4 * 1024 * 1024,
                },
            }))
            .unwrap();
        assert!(matches!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Attach { .. }
        ));
        connection
            .send(&ServerMessage::Attached {
                snapshot: DaemonSnapshot {
                    daemon_instance_id: instance,
                    sequence: 0,
                    generation: 0,
                    workspace: None,
                    project_profile: ProjectProfileSummary::NotLoaded,
                    bitbake: BitBakeState {
                        lifecycle: LifecycleState::Disconnected,
                        version: None,
                        capabilities: vec![],
                        diagnostic: None,
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
                },
                replayed_through: 0,
            })
            .unwrap();
        let ClientMessage::PtyResize(resize) = connection.receive::<ClientMessage>().unwrap()
        else {
            panic!("the running owned session must resize");
        };
        assert_eq!(resize.session_id.0, 41);
        assert_eq!(resize.writer_epoch, 9);
        assert_eq!(
            resize.dimensions,
            TerminalDimensions {
                columns: 90,
                rows: 30
            }
        );
        // Every rejected state must leave this as the next wire frame.
        assert_eq!(
            connection.receive::<ClientMessage>().unwrap(),
            ClientMessage::Detach
        );
        connection.send(&ServerMessage::Detaching).unwrap();
    });
    let mut transport = DaemonClientTransport::connect_at(
        &paths,
        ClientId([7; 16]),
        "resize-test".into(),
        Duration::from_secs(5),
    )
    .unwrap();
    transport
        .attach(
            None,
            Subscription {
                state: true,
                jobs: false,
                logs: false,
                pty_sessions: vec![],
            },
            None,
        )
        .unwrap();
    let mut runtime = InteractiveDaemonRuntime {
        transport,
        replica: DaemonClientSnapshot::default(),
        local_build_dir: None,
        next_request: 1,
        last_pty_resize: None,
        pending_terminal_completions: vec![],
        terminal_completions: vec![],
    };
    let mut app = App::new(8, 1_000);
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 41,
            name: "shell".into(),
            lifecycle: ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 41,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([7; 16]),
            writer_epoch: 9,
            exit_code: None,
            restartable: true,
        });
    assert!(
        runtime
            .resize_selected_terminal(
                &app,
                yoctui_model::PtyDimensions {
                    columns: 90,
                    rows: 30
                }
            )
            .unwrap()
    );
    assert!(
        !runtime
            .resize_selected_terminal(
                &app,
                yoctui_model::PtyDimensions {
                    columns: 90,
                    rows: 30
                }
            )
            .unwrap()
    );
    for lifecycle in [
        ClientDaemonLifecycle::Disconnected,
        ClientDaemonLifecycle::Connecting,
        ClientDaemonLifecycle::Stopping,
        ClientDaemonLifecycle::Exited,
        ClientDaemonLifecycle::Failed,
        ClientDaemonLifecycle::Lost,
    ] {
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        assert!(
            !runtime
                .resize_selected_terminal(
                    &app,
                    yoctui_model::PtyDimensions {
                        columns: 91,
                        rows: 31
                    }
                )
                .unwrap()
        );
        assert_eq!(runtime.last_pty_resize, None);
        assert_eq!(runtime.next_request, 2);
    }
    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Running;
    for status in [
        yoctui_model::ClientReplicaStatus::Stale,
        yoctui_model::ClientReplicaStatus::Disconnected,
    ] {
        app.daemon.status = status;
        assert!(
            !runtime
                .resize_selected_terminal(
                    &app,
                    yoctui_model::PtyDimensions {
                        columns: 91,
                        rows: 31
                    }
                )
                .unwrap()
        );
    }
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.pty_details[0].writer = Some([8; 16]);
    assert!(
        !runtime
            .resize_selected_terminal(
                &app,
                yoctui_model::PtyDimensions {
                    columns: 91,
                    rows: 31
                }
            )
            .unwrap()
    );
    assert_eq!(runtime.next_request, 2);
    runtime.transport.detach().unwrap();
    server.join().unwrap();
}
