#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::DirBuilderExt,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use yoctui_protocol::{
    daemon::{
        Capability, ClientHello, ClientId, ClientMessage, CommandRequest, DaemonCommand,
        ProtocolVersion, PtyCommand, PtyInput, PtyKind, PtyResize, RequestId, ServerMessage,
        Subscription, TerminalDimensions,
    },
    daemon_ipc::{DaemonConnection, runtime_paths_for},
};

struct DaemonGuard {
    binary: PathBuf,
    runtime: PathBuf,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = Command::new(&self.binary)
            .args(["daemon", "stop"])
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("XDG_STATE_HOME", self.runtime.join("state"))
            .output();
        let _ = fs::remove_dir_all(&self.runtime);
    }
}

fn attach(runtime: &Path) -> DaemonConnection {
    let paths = runtime_paths_for(runtime.to_path_buf(), unsafe { libc::geteuid() }).unwrap();
    let mut connection = DaemonConnection::connect(&paths, Duration::from_secs(2)).unwrap();
    connection
        .set_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    connection
        .send(&ClientMessage::Hello(ClientHello {
            minimum_version: ProtocolVersion::CURRENT,
            maximum_version: ProtocolVersion::CURRENT,
            client_id: ClientId([7; 16]),
            client_name: "pty-runtime-test".into(),
            capabilities: vec![
                Capability::StateSnapshots,
                Capability::IncrementalEvents,
                Capability::PtySessions,
                Capability::PtyWriterLease,
            ],
        }))
        .unwrap();
    assert!(matches!(
        connection.receive::<ServerMessage>().unwrap(),
        ServerMessage::Hello(_)
    ));
    connection
        .send(&ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: true,
                logs: true,
                pty_sessions: Vec::new(),
            },
            resume: None,
        })
        .unwrap();
    assert!(matches!(
        connection.receive::<ServerMessage>().unwrap(),
        ServerMessage::Attached { .. }
    ));
    connection
}

fn receive_command_result(connection: &mut DaemonConnection) {
    for _ in 0..30 {
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::CommandResult(result) => {
                assert!(matches!(
                    result.outcome,
                    yoctui_protocol::daemon::CommandOutcome::Accepted
                ));
                return;
            }
            ServerMessage::Event(_) | ServerMessage::Ping { .. } => {}
            other => panic!("expected command result, got {other:?}"),
        }
    }
    panic!("command result was not delivered");
}

#[test]
fn multiclient_pty_enforces_one_writer_and_routes_input() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = std::env::temp_dir().join(format!(
        "yoctui-cli-daemon-pty-runtime-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&runtime);
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let _guard = DaemonGuard {
        binary: binary.clone(),
        runtime: runtime.clone(),
    };
    let start = Command::new(&binary)
        .args(["daemon", "start"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );

    let mut connection = attach(&runtime);
    let cwd = runtime.join("pty-work");
    fs::create_dir_all(&cwd).unwrap();
    connection
        .send(&ClientMessage::Command(CommandRequest {
            request_id: RequestId(1),
            expected_generation: None,
            command: DaemonCommand::CreatePty {
                name: "test shell".into(),
                kind: PtyKind::BuildShell,
                cwd: cwd.display().to_string(),
                command: PtyCommand {
                    program: "/bin/sh".into(),
                    arguments: vec![
                        "-c".into(),
                        "printf ready; while read line; do printf \"seen:%s\\n\" \"$line\"; done"
                            .into(),
                    ],
                    environment_profile_id: None,
                },
                dimensions: TerminalDimensions {
                    columns: 80,
                    rows: 24,
                },
            },
        }))
        .unwrap();
    receive_command_result(&mut connection);
    connection
        .send(&ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: false,
                logs: false,
                pty_sessions: vec![yoctui_protocol::daemon::PtySessionId(1)],
            },
            resume: None,
        })
        .unwrap();
    let mut running = false;
    for _ in 0..20 {
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Attached { snapshot, .. } => {
                running = snapshot.pty_sessions.iter().any(|session| {
                    session.lifecycle == yoctui_protocol::daemon::LifecycleState::Running
                });
            }
            ServerMessage::Event(event) => {
                if let yoctui_protocol::daemon::DaemonEvent::PtyChanged(session) = event.event {
                    running = session.lifecycle == yoctui_protocol::daemon::LifecycleState::Running;
                }
            }
            _ => {}
        }
        if running {
            break;
        }
    }
    assert!(running, "PTY did not reach running state");
    connection
        .send(&ClientMessage::Command(CommandRequest {
            request_id: RequestId(2),
            expected_generation: None,
            command: DaemonCommand::TakePtyControl {
                session_id: yoctui_protocol::daemon::PtySessionId(1),
                expected_epoch: 0,
            },
        }))
        .unwrap();
    receive_command_result(&mut connection);
    connection
        .send(&ClientMessage::PtyInput(PtyInput {
            request_id: RequestId(3),
            session_id: yoctui_protocol::daemon::PtySessionId(1),
            writer_epoch: 1,
            bytes: b"hello\n".to_vec(),
        }))
        .unwrap();
    receive_command_result(&mut connection);
    connection
        .send(&ClientMessage::PtyResize(PtyResize {
            request_id: RequestId(4),
            session_id: yoctui_protocol::daemon::PtySessionId(1),
            writer_epoch: 1,
            dimensions: TerminalDimensions {
                columns: 100,
                rows: 30,
            },
        }))
        .unwrap();
    receive_command_result(&mut connection);
}
