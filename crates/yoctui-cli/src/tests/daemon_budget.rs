use super::*;
use yoctui_model::{DaemonGlobalState, DaemonModelInstanceId, DaemonStateLimits};
use yoctui_protocol::daemon::{
    DaemonBuildEvent, DaemonEvent, DaemonSnapshotError, ProtocolErrorCode, RequestId,
};

fn journal() -> DaemonSnapshotJournal {
    let state = DaemonGlobalState::new(
        DaemonModelInstanceId([1; 16]),
        1,
        "budget-test".into(),
        DaemonStateLimits::default(),
    )
    .unwrap();
    DaemonSnapshotJournal::new(
        crate::daemon_protocol_snapshot(&state),
        DaemonSnapshotLimits::default(),
    )
    .unwrap()
}

#[test]
fn oversized_updates_recover_without_hiding_unrelated_errors() {
    let mut journal = journal();
    for error in [
        DaemonSnapshotError::SnapshotTooLarge {
            actual: 4_692_964,
            maximum: MAX_FRAME_BYTES,
        },
        DaemonSnapshotError::EventTooLarge {
            actual: 5_000_000,
            maximum: MAX_FRAME_BYTES,
        },
    ] {
        recover(Err(error.into()), &mut journal).unwrap();
        assert!(
            journal
                .snapshot()
                .recent_logs
                .last()
                .unwrap()
                .message
                .contains("remain active")
        );
        journal
            .publish(DaemonEvent::Build(DaemonBuildEvent::Completed {
                success: true,
                exit_code: Some(0),
                finished_unix_ms: Some(10),
            }))
            .unwrap();
    }
    assert!(
        recover(
            Err(anyhow::anyhow!("unrelated internal failure")),
            &mut journal
        )
        .is_err()
    );
}

#[test]
fn oversized_reply_is_rejected_and_same_socket_continues_both_directions() {
    use std::os::unix::fs::PermissionsExt;
    use yoctui_protocol::daemon_ipc::runtime_paths_for;
    let runtime = tempfile::tempdir().unwrap();
    std::fs::set_permissions(runtime.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
    let paths =
        runtime_paths_for(runtime.path().to_path_buf(), unsafe { libc::geteuid() }).unwrap();
    let listener = DaemonListener::bind(&paths).unwrap();
    let mut peer = DaemonConnection::connect(&paths, Duration::from_secs(1)).unwrap();
    let mut server = listener.accept(Duration::from_secs(1)).unwrap();
    peer.set_timeout(Some(Duration::from_secs(1))).unwrap();
    server.set_timeout(Some(Duration::from_secs(1))).unwrap();
    let mut journal = journal();
    send_command_result(
        &mut server,
        &mut journal,
        CommandResult {
            request_id: RequestId(1),
            outcome: CommandOutcome::Rejected {
                code: ProtocolErrorCode::Conflict,
                message: "x".repeat(MAX_FRAME_BYTES),
                current_generation: 0,
            },
        },
    )
    .unwrap();
    assert!(matches!(
        peer.receive::<ServerMessage>().unwrap(),
        ServerMessage::CommandResult(CommandResult {
            request_id: RequestId(1),
            outcome: CommandOutcome::Rejected {
                code: ProtocolErrorCode::MessageTooLarge,
                ..
            },
        })
    ));
    // Neither a size rejection nor reporting it closes/poisons the stream.
    recover(
        Err(DaemonSnapshotError::SnapshotTooLarge {
            actual: 4_692_964,
            maximum: MAX_FRAME_BYTES,
        }
        .into()),
        &mut journal,
    )
    .unwrap();
    peer.send(&ClientMessage::Pong { nonce: 42 }).unwrap();
    assert!(matches!(
        server.receive::<ClientMessage>().unwrap(),
        ClientMessage::Pong { nonce: 42 }
    ));
    send_command_result(
        &mut server,
        &mut journal,
        CommandResult {
            request_id: RequestId(2),
            outcome: CommandOutcome::Completed,
        },
    )
    .unwrap();
    assert!(matches!(
        peer.receive::<ServerMessage>().unwrap(),
        ServerMessage::CommandResult(CommandResult {
            request_id: RequestId(2),
            outcome: CommandOutcome::Completed
        })
    ));
    server
        .send(&ServerMessage::Snapshot(journal.snapshot().clone()))
        .unwrap();
    assert!(matches!(
        peer.receive::<ServerMessage>().unwrap(),
        ServerMessage::Snapshot(_)
    ));
}
