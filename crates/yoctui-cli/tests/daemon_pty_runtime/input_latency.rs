//! Real isolated daemon/PTY regression; this is not evidence of a QEMU guest boot.
use super::*;
use std::os::unix::fs::PermissionsExt;
use std::time::Instant;
use yoctui_protocol::daemon::{CommandOutcome, DaemonEvent, PtySessionId};

#[test]
fn typing_during_console_output_does_not_replace_the_entire_replica() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let directory = tempfile::Builder::new()
        .prefix("yoctui-input-latency-")
        .tempdir()
        .unwrap();
    let runtime = directory.path().to_path_buf();
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
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
            name: "console echo fixture".into(), kind: PtyKind::QemuConsole,
            cwd: runtime.display().to_string(),
            command: PtyCommand {
                program: "/usr/bin/python3".into(), environment_profile_id: None,
                arguments: vec!["-c".into(), "import os,threading,time,tty\ntty.setraw(0)\ndef output():\n while True:\n  os.write(1,b'console tick\\r\\n');time.sleep(.02)\nthreading.Thread(target=output,daemon=True).start()\nwhile True:\n b=os.read(0,1)\n if not b: break\n os.write(1,b'ECHO:'+b+b'\\r\\n')".into()],
            }, dimensions: TerminalDimensions { columns: 80, rows: 24 },
        },
    })).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    // Drain initial creation before acquiring the writer lease.
    let mut running = false;
    let mut created = false;
    while !(running && created) {
        assert!(Instant::now() < deadline);
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Event(event) => {
                if let DaemonEvent::PtyChanged(pty) = event.event {
                    running |= pty.lifecycle == yoctui_protocol::daemon::LifecycleState::Running;
                }
            }
            ServerMessage::CommandResult(result) if result.request_id == RequestId(1) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                created = true;
            }
            _ => {}
        }
    }
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
    let mut sequence = loop {
        if let ServerMessage::Attached { snapshot, .. } =
            connection.receive::<ServerMessage>().unwrap()
        {
            break snapshot.sequence;
        }
    };
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
    let mut writer = false;
    let mut accepted = false;
    let mut ready = false;
    let mut startup_output = Vec::new();
    // Started means the PTY child was spawned, not that Python has configured
    // raw input. Observe its first tick before sending bytes without a newline;
    // otherwise canonical-mode input can strand the burst on a busy test host.
    while !(writer && accepted && ready) {
        assert!(Instant::now() < deadline);
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Snapshot(snapshot) => {
                sequence = snapshot.sequence;
                writer |= snapshot
                    .pty_sessions
                    .iter()
                    .any(|pty| pty.writer_epoch == 1);
            }
            ServerMessage::Event(event) => {
                assert_eq!(event.sequence, sequence + 1);
                sequence = event.sequence;
                if let DaemonEvent::PtyChanged(pty) = event.event {
                    writer |= pty.writer_epoch == 1;
                } else if let DaemonEvent::PtyOutput { bytes, .. } = event.event {
                    startup_output.extend(bytes);
                    ready |= startup_output
                        .windows(b"console tick".len())
                        .any(|bytes| bytes == b"console tick");
                }
            }
            ServerMessage::CommandResult(result) if result.request_id == RequestId(2) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                accepted = true;
            }
            _ => {}
        }
    }
    // Backlogged console events must not force a full workspace snapshot just
    // because another key arrives. Input still uses the authoritative epoch.
    std::thread::sleep(Duration::from_millis(80));
    let started = Instant::now();
    for id in 10..30 {
        connection
            .send(&ClientMessage::PtyInput(PtyInput {
                request_id: RequestId(id),
                session_id: PtySessionId(1),
                writer_epoch: 1,
                bytes: vec![b'x'],
            }))
            .unwrap();
    }
    let mut acknowledgements = 0;
    let mut echo = false;
    let mut output = Vec::new();
    while acknowledgements < 20 || !echo {
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "input burst stalled: acknowledgements={acknowledgements}, echo={echo}"
        );
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Snapshot(_) => {
                panic!("typing caused an unnecessary full-replica replacement")
            }
            ServerMessage::Event(event) => {
                assert_eq!(event.sequence, sequence + 1, "input broke ordered replay");
                sequence = event.sequence;
                if let DaemonEvent::PtyOutput { bytes, .. } = event.event {
                    output.extend(bytes);
                    echo |= output.windows(6).any(|bytes| bytes == b"ECHO:x");
                }
            }
            ServerMessage::CommandResult(result) if (10..30).contains(&result.request_id.0) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                acknowledgements += 1;
            }
            _ => {}
        }
    }
    eprintln!(
        "20 real PTY inputs acknowledged with echo in {:?}",
        started.elapsed()
    );
    // A stale writer must still be rejected on the fast path.
    connection
        .send(&ClientMessage::PtyInput(PtyInput {
            request_id: RequestId(100),
            session_id: PtySessionId(1),
            writer_epoch: 0,
            bytes: vec![b'z'],
        }))
        .unwrap();
    loop {
        if let ServerMessage::CommandResult(result) = connection.receive::<ServerMessage>().unwrap()
            && result.request_id == RequestId(100)
        {
            assert!(matches!(result.outcome, CommandOutcome::Rejected { .. }));
            break;
        }
    }
}
