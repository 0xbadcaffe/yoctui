use super::*;

#[test]
fn qemu_adapter_normalizes_forced_cancellation_and_loss() {
    let id = QemuSessionId(11);
    let timestamp = SystemTime::UNIX_EPOCH;
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            timestamp,
        ),
        vec![
            Action::AppendQemuSessionOutput {
                id,
                stream: QemuOutputStream::Stderr,
                line: "runqemu cancellation required forced termination".into(),
                truncated: false,
                timestamp,
            },
            Action::CancelQemuSession {
                id,
                exit_code: Some(137),
                finished_at: timestamp,
            }
        ]
    );
    assert_eq!(
        qemu_actions_for_runner_event(
            id,
            QemuRunnerEvent::Lost {
                message: "event channel lost".into(),
            },
            timestamp,
        ),
        vec![Action::LoseQemuSession {
            id,
            message: "event channel lost".into(),
            finished_at: timestamp,
        }]
    );
}
