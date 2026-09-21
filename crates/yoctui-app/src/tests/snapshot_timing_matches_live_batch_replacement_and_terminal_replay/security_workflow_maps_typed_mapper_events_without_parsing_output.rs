use super::*;

#[test]
fn security_workflow_maps_typed_mapper_events_without_parsing_output() {
    let id = yoctui_model::SecuritySessionId(7);
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Started { id },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::SessionRunning(id))]
    );
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Output {
                id,
                stream: SecurityOutputStream::Stderr,
                line: "package=busybox product=busybox".into(),
                truncated: true,
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::SessionOutput {
            id,
            stream: SecurityOutputStream::Stderr,
            line: "package=busybox product=busybox".into(),
            truncated: true,
        })]
    );
    assert!(matches!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::TimedOut {
                id,
                forced: true,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [
            Action::Security(SecurityAction::SessionOutput { id: output_id, .. }),
            Action::Security(SecurityAction::TimeoutSession { id: timeout_id, .. }),
        ] if *output_id == id && *timeout_id == id
    ));
    assert_eq!(
        security_actions_for_mapper_event(
            SecurityMapperRunnerEvent::Lost {
                id,
                message: "worker channel closed".into(),
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::Security(SecurityAction::LoseSession {
            id,
            message: "worker channel closed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        })]
    );
}
