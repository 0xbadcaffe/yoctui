use super::*;

#[test]
fn wic_model_normalizes_typed_session_events_without_parsing_output() {
    let id = WicSessionId(7);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        wic_actions_for_session_event(id, WicSessionEvent::Starting, timestamp),
        vec![Action::WicSessionStarting {
            id,
            started_at: timestamp
        }]
    );
    assert_eq!(
        wic_actions_for_session_event(
            id,
            WicSessionEvent::Output {
                stream: WicOutputStream::Stderr,
                line: "raw adapter text".into(),
                truncated: true,
            },
            timestamp,
        ),
        vec![Action::AppendWicSessionOutput {
            id,
            stream: WicOutputStream::Stderr,
            line: "raw adapter text".into(),
            truncated: true,
            timestamp,
        }]
    );
    assert_eq!(
        wic_actions_for_session_event(
            id,
            WicSessionEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            timestamp,
        ),
        vec![
            Action::AppendWicSessionOutput {
                id,
                stream: WicOutputStream::Stderr,
                line: "Wic cancellation required forced termination".into(),
                truncated: false,
                timestamp,
            },
            Action::CancelWicSession {
                id,
                exit_code: Some(137),
                finished_at: timestamp,
            }
        ]
    );
}
