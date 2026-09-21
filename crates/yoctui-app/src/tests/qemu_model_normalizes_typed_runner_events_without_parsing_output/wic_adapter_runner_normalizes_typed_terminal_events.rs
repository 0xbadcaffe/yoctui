use super::*;

#[test]
fn wic_adapter_runner_normalizes_typed_terminal_events() {
    let id = WicSessionId(8);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        wic_actions_for_runner_event(
            id,
            WicRunnerEvent::Failed {
                message: "failed".into(),
                exit_code: Some(9),
            },
            timestamp,
        ),
        vec![Action::FailWicSession {
            id,
            message: "failed".into(),
            exit_code: Some(9),
            finished_at: timestamp,
        }]
    );
}
