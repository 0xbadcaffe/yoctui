use super::*;

#[test]
fn test_runner_events_map_once_with_stream_and_terminal_meaning() {
    let id = yoctui_model::TestSessionId(9);
    assert_eq!(
        test_actions_for_runner_event(id, TestRunnerEvent::Started, SystemTime::UNIX_EPOCH),
        [
            Action::TestSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH,
            },
            Action::TestSessionRunning { id },
        ]
    );
    assert_eq!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Output {
                stream: yoctui_model::TestOutputStream::Stderr,
                line: "warning".into(),
                truncated: true,
            },
            SystemTime::UNIX_EPOCH,
        ),
        [Action::AppendTestSessionOutput {
            id,
            stream: yoctui_model::TestOutputStream::Stderr,
            line: "warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        }]
    );
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Completed {
                exit_code: Some(0),
                result_paths: vec!["/build/testresults.json".into()],
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::CompleteTestSession {
            id: yoctui_model::TestSessionId(9),
            exit_code: 0,
            result_paths,
            ..
        }] if result_paths == &[PathBuf::from("/build/testresults.json")]
    ));
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::TimedOut {
                forced: true,
                exit_code: None,
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::TimeoutTestSession {
            id: yoctui_model::TestSessionId(9),
            forced: true,
            exit_code: None,
            ..
        }]
    ));
    assert_eq!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Cancelled {
                forced: true,
                exit_code: Some(137),
            },
            SystemTime::UNIX_EPOCH,
        )
        .len(),
        2,
        "forced cancellation retains both its warning and terminal action"
    );
    assert!(matches!(
        test_actions_for_runner_event(
            id,
            TestRunnerEvent::Lost {
                message: "channel closed".into(),
            },
            SystemTime::UNIX_EPOCH,
        )
        .as_slice(),
        [Action::LoseTestSession { message, .. }] if message == "channel closed"
    ));
}
