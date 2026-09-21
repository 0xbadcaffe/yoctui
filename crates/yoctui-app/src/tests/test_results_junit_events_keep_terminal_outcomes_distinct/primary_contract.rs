use super::*;

#[test]
fn test_results_junit_events_keep_terminal_outcomes_distinct() {
    let result = yoctui_model::TestResultIdentity::new(
        "/results/testresults.json".into(),
        10,
        SystemTime::UNIX_EPOCH,
        "result".into(),
    )
    .unwrap();
    let request = yoctui_model::TestJunitExportRequest {
        generation: 8,
        result,
        destination: "/exports/results.xml".into(),
    };
    let operation = TestResultOperation::Junit(request.clone());
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Completed {
                operation: operation.clone(),
                exit_code: Some(0),
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportSucceeded {
            request: request.clone()
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::TimedOut {
                operation: operation.clone(),
                forced: true,
                exit_code: None,
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportTimedOut {
            request: request.clone()
        }]
    );
    assert_eq!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::Lost {
                operation: Some(operation),
                message: "worker channel closed".into(),
            },
            None,
            Vec::new(),
        ),
        [Action::TestJunitExportLost {
            request,
            message: "worker channel closed".into(),
        }]
    );
    assert!(
        test_result_actions_for_runner_event(
            TestResultRunnerEvent::CancellationRejected {
                message: "not running".into(),
            },
            None,
            Vec::new(),
        )
        .is_empty()
    );
}
