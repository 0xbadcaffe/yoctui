pub fn test_actions_for_runner_event(
    id: yoctui_model::TestSessionId,
    event: TestRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        TestRunnerEvent::Started => vec![
            Action::TestSessionStarting {
                id,
                started_at: timestamp,
            },
            Action::TestSessionRunning { id },
        ],
        TestRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendTestSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        }],
        TestRunnerEvent::Completed {
            exit_code,
            result_paths,
        } => vec![Action::CompleteTestSession {
            id,
            exit_code: exit_code.unwrap_or(0),
            result_paths,
            finished_at: timestamp,
        }],
        TestRunnerEvent::Failed { exit_code } => vec![Action::FailTestSession {
            id,
            message: exit_code.map_or_else(
                || "Testing process exited unsuccessfully without an exit code".into(),
                |code| format!("Testing process exited unsuccessfully with exit code {code}"),
            ),
            exit_code,
            finished_at: timestamp,
        }],
        TestRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendTestSessionOutput {
                    id,
                    stream: yoctui_model::TestOutputStream::Stderr,
                    line: "Testing cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelTestSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        TestRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectTestSessionCancellation { id, message }]
        }
        TestRunnerEvent::TimedOut { forced, exit_code } => {
            vec![Action::TimeoutTestSession {
                id,
                forced,
                exit_code,
                finished_at: timestamp,
            }]
        }
        TestRunnerEvent::Lost { message } => vec![Action::LoseTestSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}

pub fn test_results_import_action(response: TestResultImportResponse) -> Action {
    Action::TestResultsLoaded {
        request: response.request,
        records: response.records,
        limitations: response.limitations,
    }
}

pub fn test_result_actions_for_runner_event(
    event: TestResultRunnerEvent,
    comparison: Option<TestComparison>,
    limitations: Vec<String>,
) -> Vec<Action> {
    match event {
        TestResultRunnerEvent::Completed { operation, .. } => match operation {
            TestResultOperation::Comparison(request) => match comparison {
                Some(comparison) => vec![Action::TestComparisonLoaded {
                    request,
                    comparison,
                    limitations,
                }],
                None => vec![Action::TestComparisonFailed {
                    request,
                    message: "resulttool completed without a typed comparison".into(),
                }],
            },
            TestResultOperation::Junit(request) => {
                vec![Action::TestJunitExportSucceeded { request }]
            }
        },
        TestResultRunnerEvent::Failed {
            operation,
            exit_code,
        } => {
            let message = exit_code.map_or_else(
                || "resulttool exited unsuccessfully without an exit code".into(),
                |code| format!("resulttool exited unsuccessfully with exit code {code}"),
            );
            test_result_failed_action(operation, message)
        }
        TestResultRunnerEvent::Cancelled { operation, .. } => match operation {
            TestResultOperation::Comparison(request) => {
                vec![Action::TestComparisonCancelled { request }]
            }
            TestResultOperation::Junit(request) => {
                vec![Action::TestJunitExportCancelled { request }]
            }
        },
        TestResultRunnerEvent::TimedOut { operation, .. } => match operation {
            TestResultOperation::Comparison(request) => {
                vec![Action::TestComparisonTimedOut { request }]
            }
            TestResultOperation::Junit(request) => {
                vec![Action::TestJunitExportTimedOut { request }]
            }
        },
        TestResultRunnerEvent::Lost {
            operation: Some(operation),
            message,
        } => match operation {
            TestResultOperation::Comparison(request) => {
                vec![Action::TestComparisonLost { request, message }]
            }
            TestResultOperation::Junit(request) => {
                vec![Action::TestJunitExportLost { request, message }]
            }
        },
        TestResultRunnerEvent::Started { .. }
        | TestResultRunnerEvent::Output { .. }
        | TestResultRunnerEvent::CancellationRejected { .. }
        | TestResultRunnerEvent::Lost {
            operation: None, ..
        } => Vec::new(),
    }
}

pub(crate) fn test_result_failed_action(
    operation: TestResultOperation,
    message: String,
) -> Vec<Action> {
    match operation {
        TestResultOperation::Comparison(request) => {
            vec![Action::TestComparisonFailed { request, message }]
        }
        TestResultOperation::Junit(request) => {
            vec![Action::TestJunitExportFailed { request, message }]
        }
    }
}
