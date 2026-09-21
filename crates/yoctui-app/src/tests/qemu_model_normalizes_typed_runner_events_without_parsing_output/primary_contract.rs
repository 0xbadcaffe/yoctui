use super::*;

#[test]
fn qemu_model_normalizes_typed_runner_events_without_parsing_output() {
    let id = QemuSessionId(7);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        qemu_actions_for_runner_event(id, QemuRunnerEvent::Starting, timestamp),
        vec![Action::QemuSessionStarting {
            id,
            started_at: timestamp
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Output {
                stream: QemuRunnerOutputStream::Stderr,
                line: "verbatim runner output".into(),
                truncated: true,
            },
            timestamp,
        ),
        vec![Action::AppendQemuSessionOutput {
            id,
            stream: QemuOutputStream::Stderr,
            line: "verbatim runner output".into(),
            truncated: true,
            timestamp,
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Failed {
                message: "spawn failed".into(),
                exit_code: Some(127),
            },
            timestamp,
        ),
        vec![Action::FailQemuSession {
            id,
            message: "spawn failed".into(),
            exit_code: Some(127),
            finished_at: timestamp,
        }]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::CancellationRejected {
                message: "not running".into(),
            },
            timestamp,
        ),
        vec![Action::RejectQemuSessionCancellation {
            id,
            message: "not running".into(),
        }]
    );
}
