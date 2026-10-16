use super::*;
use std::time::Instant;
use yoctui_protocol::daemon::{CommandOutcome, DaemonEvent, PtySessionId};

/// Used with a real PTY for shell, QEMU-console and SSH-console session kinds.
pub(super) fn assert_ordered_after_viewport(connection: &mut DaemonConnection) {
    connection
        .send(&ClientMessage::PtyViewport(PtyViewport {
            request_id: RequestId(5),
            session_id: PtySessionId(1),
            scrollback_offset: 0,
        }))
        .unwrap();
    let mut sequence = None;
    let mut saw_screen = false;
    loop {
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Snapshot(snapshot) => sequence = Some(snapshot.sequence),
            ServerMessage::Event(event) => {
                if let Some(prior) = sequence {
                    assert_eq!(event.sequence, prior + 1, "viewport event ordering");
                }
                sequence = Some(event.sequence);
                saw_screen |= matches!(event.event, DaemonEvent::PtyScreen(_));
            }
            ServerMessage::CommandResult(result) if result.request_id == RequestId(5) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                break;
            }
            _ => {}
        }
    }
    assert!(saw_screen, "viewport did not return a screen");
    connection
        .send(&ClientMessage::PtyInput(PtyInput {
            request_id: RequestId(55),
            session_id: PtySessionId(1),
            writer_epoch: 1,
            bytes: b"after-search\n".to_vec(),
        }))
        .unwrap();
    let started = Instant::now();
    let mut accepted = false;
    let mut output = Vec::new();
    let marker = b"seen:after-search";
    while !accepted || !output.windows(marker.len()).any(|bytes| bytes == marker) {
        assert!(
            started.elapsed() < Duration::from_secs(3),
            "writer lost after viewport"
        );
        match connection.receive::<ServerMessage>().unwrap() {
            ServerMessage::Event(event) => {
                assert_eq!(
                    event.sequence,
                    sequence.unwrap() + 1,
                    "viewport replayed a directly delivered event"
                );
                sequence = Some(event.sequence);
                if let DaemonEvent::PtyOutput { bytes, .. } = event.event {
                    output.extend(bytes);
                }
            }
            ServerMessage::CommandResult(result) if result.request_id == RequestId(55) => {
                assert_eq!(result.outcome, CommandOutcome::Accepted);
                accepted = true;
            }
            ServerMessage::Snapshot(_) | ServerMessage::ResyncRequired { .. } => {
                panic!("viewport forced an unnecessary resynchronization");
            }
            _ => {}
        }
    }
}
