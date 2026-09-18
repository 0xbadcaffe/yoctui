use super::*;
use yoctui_protocol::daemon::{CommandOutcome, JobId, ProtocolErrorCode, PtySessionId};

#[test]
fn rejected_command_families_keep_one_reply_and_preserve_the_client_connection() {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_yoctui"));
    let runtime = unique_temp_root("yoctui-command-routing");
    fs::DirBuilder::new().mode(0o700).create(&runtime).unwrap();
    let _guard = DaemonGuard {
        binary: binary.clone(),
        runtime: runtime.clone(),
    };
    let started = Command::new(&binary)
        .args(["daemon", "start"])
        .env("XDG_RUNTIME_DIR", &runtime)
        .env("XDG_STATE_HOME", runtime.join("state"))
        .env_remove("BUILDDIR")
        .output()
        .unwrap();
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let mut connection = connect_and_attach(&runtime, ClientId([31; 16]));
    let commands = [
        DaemonCommand::StartBuild {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        },
        DaemonCommand::CancelJob { job_id: JobId(999) },
        DaemonCommand::CancelRaw {
            request_id: "missing".into(),
        },
        DaemonCommand::CancelSdk { session_id: 999 },
        DaemonCommand::CancelQemu { session_id: 999 },
        DaemonCommand::CancelWic { session_id: 999 },
        DaemonCommand::CancelTestSession { session_id: 999 },
        DaemonCommand::CancelQaLayerCheck { session_id: 999 },
        DaemonCommand::CancelSecurityPackageMap { session_id: 999 },
        DaemonCommand::CancelMaintenance { session_id: 999 },
        DaemonCommand::RenamePty {
            session_id: PtySessionId(999),
            name: "missing".into(),
        },
    ];
    for (index, command) in commands.into_iter().enumerate() {
        let request_id = RequestId(index as u64 + 1);
        connection
            .send(&ClientMessage::Command(CommandRequest {
                request_id,
                expected_generation: None,
                command,
            }))
            .unwrap();
        let reply = loop {
            match connection.receive::<ServerMessage>().unwrap() {
                ServerMessage::Event(_) => continue,
                ServerMessage::CommandResult(result) => break result,
                other => panic!("unexpected command response: {other:?}"),
            }
        };
        assert_eq!(reply.request_id, request_id);
        if index == 0 {
            assert!(
                matches!(&reply.outcome, CommandOutcome::Rejected { code: ProtocolErrorCode::MalformedMessage, message, .. } if message.contains("build-directory authority"))
            );
        }
        assert!(
            matches!(reply.outcome, CommandOutcome::Rejected { code, .. } if code != ProtocolErrorCode::UnsupportedCapability)
        );
        connection
            .send(&ClientMessage::Attach {
                workspace: None,
                resume: None,
                subscription: Subscription {
                    state: true,
                    jobs: true,
                    logs: true,
                    pty_sessions: Vec::new(),
                },
            })
            .unwrap();
        loop {
            match connection.receive::<ServerMessage>().unwrap() {
                ServerMessage::Event(_) => continue,
                ServerMessage::Attached { .. } => break,
                other => panic!("duplicate reply or lost client continuity: {other:?}"),
            }
        }
    }
}
