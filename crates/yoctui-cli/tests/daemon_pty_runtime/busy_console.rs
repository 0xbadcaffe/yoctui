use super::*;
use std::time::Instant;
use yoctui_protocol::daemon::{CommandOutcome, DaemonEvent, PtySessionId};

#[test]
fn busy_console_writer_control_is_prompt_and_replica_events_remain_contiguous() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = std::env::temp_dir().join(format!("yoctui-busy-console-{}", std::process::id()));
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let _guard = DaemonGuard {
        binary: binary.clone(),
        runtime: runtime.clone(),
    };
    let start = Command::new(&binary)
        .args(["daemon", "start"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .env("XDG_CONFIG_HOME", runtime.join("config"))
        .env("XDG_DATA_HOME", runtime.join("data"))
        .env_remove("BUILDDIR")
        .output()
        .unwrap();
    assert!(
        start.status.success(),
        "{}",
        String::from_utf8_lossy(&start.stderr)
    );
    let (mut connection, _) = attach(&runtime);
    connection.send(&ClientMessage::Command(CommandRequest {
        request_id: RequestId(1), expected_generation: None,
        command: DaemonCommand::CreatePty {
            name: "busy console".into(), kind: PtyKind::QemuConsole,
            cwd: runtime.display().to_string(),
            command: PtyCommand {
                program: "/usr/bin/python3".into(),
                arguments: vec!["-c".into(), "import os,time\nfor i in range(1000):\n os.write(1,b'console output '*4096+b'\\n');time.sleep(.005)\ntime.sleep(30)".into()],
                environment_profile_id: None,
            }, dimensions: TerminalDimensions { columns: 80, rows: 24 },
        },
    })).unwrap();
    receive_command_result(&mut connection);
    connection
        .send(&ClientMessage::Attach {
            workspace: None,
            subscription: Subscription {
                state: true,
                jobs: false,
                logs: false,
                pty_sessions: vec![PtySessionId(1)],
            },
            resume: None,
        })
        .unwrap();
    loop {
        if matches!(
            connection.receive::<ServerMessage>().unwrap(),
            ServerMessage::Attached { .. }
        ) {
            break;
        }
    }
    // A stalled reader plus a writer that polls in UI-sized slices exercises
    // pending event frames. This is transport/PTY evidence, not a guest boot.
    let (_slow_reader, _) = attach_client(&runtime, ClientId([8; 16]));
    std::thread::sleep(Duration::from_millis(300));
    connection
        .send(&ClientMessage::Command(CommandRequest {
            request_id: RequestId(2),
            expected_generation: None,
            command: DaemonCommand::TakePtyControl {
                session_id: PtySessionId(1),
                expected_epoch: 0,
            },
        }))
        .unwrap();
    let started = Instant::now();
    let mut accepted = false;
    let mut writer = false;
    let mut sequence = None;
    let mut frames = 0;
    while started.elapsed() < Duration::from_secs(3) && !(accepted && writer) {
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Snapshot(snapshot) => {
                sequence = Some(snapshot.sequence);
                writer |= snapshot.pty_sessions.iter().any(|session| {
                    session.writer == Some(ClientId([7; 16])) && session.writer_epoch == 1
                });
            }
            ServerMessage::Event(event) => {
                if let Some(previous) = sequence {
                    assert_eq!(event.sequence, previous + 1);
                }
                sequence = Some(event.sequence);
                if let DaemonEvent::PtyChanged(session) = event.event {
                    writer |=
                        session.writer == Some(ClientId([7; 16])) && session.writer_epoch == 1;
                }
            }
            ServerMessage::CommandResult(result) if result.request_id == RequestId(2) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                accepted = true;
            }
            ServerMessage::ResyncRequired { .. } | ServerMessage::Ping { .. } => {}
            other => panic!("unexpected busy-console frame {other:?}"),
        }
        frames += 1;
        if frames % 16 == 0 {
            std::thread::sleep(Duration::from_millis(8));
        }
    }
    assert!(
        accepted && writer,
        "writer control delayed: accepted={accepted}, writer={writer}, elapsed={:?}",
        started.elapsed()
    );
}
