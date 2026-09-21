use super::*;

#[test]
fn daemon_protocol_covers_reconnect_stale_writer_layout_and_mouse() {
    let attach = ClientMessage::Attach {
        workspace: None,
        subscription: Subscription {
            state: true,
            jobs: true,
            logs: false,
            pty_sessions: vec![PtySessionId(8)],
        },
        resume: Some(ResumeCursor {
            daemon_instance_id: DaemonInstanceId([9; 16]),
            last_sequence: 77,
        }),
    };
    assert_eq!(
        decode_frame::<ClientMessage>(&encode_frame(&attach).unwrap()).unwrap(),
        attach
    );

    let stale = ServerMessage::CommandResult(CommandResult {
        request_id: RequestId(5),
        outcome: CommandOutcome::Rejected {
            code: ProtocolErrorCode::StaleGeneration,
            message: "snapshot replaced".into(),
            current_generation: 12,
        },
    });
    assert_eq!(
        decode_frame::<ServerMessage>(&encode_frame(&stale).unwrap()).unwrap(),
        stale
    );

    for message in [
        ClientMessage::Layout {
            event: ClientLayoutEvent::AttachSession {
                pane_id: PaneId(2),
                session_id: PtySessionId(8),
            },
        },
        ClientMessage::Mouse {
            event: ServerMouseEvent {
                session_id: PtySessionId(8),
                writer_epoch: 3,
                kind: MouseEventKind::Drag,
                button: 1,
                column: 40,
                row: 12,
                modifiers: 0,
            },
        },
    ] {
        assert_eq!(
            decode_frame::<ClientMessage>(&encode_frame(&message).unwrap()).unwrap(),
            message
        );
    }
}
