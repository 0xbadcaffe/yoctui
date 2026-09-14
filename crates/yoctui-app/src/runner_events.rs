//! Runner events.
use super::*;

pub fn qa_layer_capability_action(response: QaLayerCapabilityResponse) -> Action {
    match response {
        QaLayerCapabilityResponse::Available(snapshot) => {
            Action::Qa(QaAction::LayerCapabilityLoaded(snapshot))
        }
        QaLayerCapabilityResponse::Partial(snapshot) => {
            let limitations = snapshot.limitations.clone();
            Action::Qa(QaAction::LayerCapabilityPartial {
                snapshot,
                limitations,
            })
        }
    }
}

pub fn qa_layer_runner_action(event: QaLayerRunnerEvent, timestamp: SystemTime) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    match event {
        QaLayerRunnerEvent::Started { id } => qa(QaAction::LayerSessionRunning(id)),
        QaLayerRunnerEvent::Output {
            id,
            stream,
            line,
            truncated,
        } => qa(QaAction::LayerSessionOutput {
            session: id,
            stream,
            line,
            truncated,
        }),
        QaLayerRunnerEvent::Completed {
            id,
            exit_code: Some(exit_code),
        } => qa(QaAction::CompleteLayerSession {
            session: id,
            exit_code,
            result_paths: Vec::new(),
            finished_at: timestamp,
        }),
        QaLayerRunnerEvent::Completed {
            id,
            exit_code: None,
        } => qa(QaAction::FailLayerSession {
            session: id,
            exit_code: None,
            message: "layer QA completed without an exit code".into(),
            finished_at: timestamp,
        }),
        QaLayerRunnerEvent::Failed { id, exit_code } => qa(QaAction::FailLayerSession {
            session: id,
            exit_code,
            message: exit_code.map_or_else(
                || "layer QA failed without an exit code".into(),
                |code| format!("layer QA failed with exit code {code}"),
            ),
            finished_at: timestamp,
        }),
        QaLayerRunnerEvent::CancellationRequested { .. } => None,
        QaLayerRunnerEvent::Cancelled {
            id,
            forced,
            exit_code,
        } => qa(QaAction::CancelLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at: timestamp,
        }),
        QaLayerRunnerEvent::CancellationRejected { id, message } => {
            qa(QaAction::RejectLayerCancellation {
                session: id,
                message,
            })
        }
        QaLayerRunnerEvent::TimedOut {
            id,
            forced,
            exit_code,
        } => qa(QaAction::TimeoutLayerSession {
            session: id,
            forced,
            exit_code,
            finished_at: timestamp,
        }),
        QaLayerRunnerEvent::Lost { id, message } => qa(QaAction::LoseLayerSession {
            session: id,
            message,
            finished_at: timestamp,
        }),
    }
}

pub fn qa_report_response_action(response: QaReportResponse) -> Action {
    let (reports, limitations) = match response.outcome {
        QaReportScanOutcome::Empty => (Vec::new(), Vec::new()),
        QaReportScanOutcome::Complete(reports) => (reports, Vec::new()),
        QaReportScanOutcome::Partial {
            reports,
            limitations,
        } => (reports, limitations),
    };
    Action::Qa(QaAction::ReportsLoaded {
        request: response.request,
        reports,
        limitations,
    })
}

pub fn qa_report_error_action(request: QaReportRequest, error: QaReportAdapterError) -> Action {
    let qa = |action| Action::Qa(action);
    match error {
        QaReportAdapterError::Cancelled => qa(QaAction::ReportsCancelled(request)),
        QaReportAdapterError::Timeout(_) => qa(QaAction::ReportsTimedOut(request)),
        QaReportAdapterError::WorkerLost(message) => qa(QaAction::ReportsLost { request, message }),
        error => {
            let kind = match &error {
                QaReportAdapterError::MissingPath(_) => QaReportFailureKind::Missing,
                QaReportAdapterError::PermissionDenied(_) => QaReportFailureKind::PermissionDenied,
                QaReportAdapterError::StaleReport(_) => QaReportFailureKind::Stale,
                QaReportAdapterError::MalformedReport(_)
                | QaReportAdapterError::UnsupportedPath(_)
                | QaReportAdapterError::OversizedReport(_) => QaReportFailureKind::Malformed,
                QaReportAdapterError::InvalidRequest(_)
                | QaReportAdapterError::UnsafePath(_)
                | QaReportAdapterError::SymlinkPath(_)
                | QaReportAdapterError::EscapePath(_)
                | QaReportAdapterError::Io(_)
                | QaReportAdapterError::NoUsableReports(_) => QaReportFailureKind::Failed,
                QaReportAdapterError::Timeout(_)
                | QaReportAdapterError::Cancelled
                | QaReportAdapterError::WorkerLost(_) => unreachable!(),
            };
            qa(QaAction::ReportsFailed {
                request,
                kind,
                message: error.to_string(),
            })
        }
    }
}

pub fn qa_task_capability_action(response: QaTaskCapabilityResponse) -> Action {
    match response {
        QaTaskCapabilityResponse::Available(snapshot) => {
            Action::Qa(QaAction::CapabilityLoaded(snapshot))
        }
        QaTaskCapabilityResponse::Partial(snapshot) => {
            let limitations = snapshot.limitations.clone();
            Action::Qa(QaAction::CapabilityPartial {
                snapshot,
                limitations,
            })
        }
    }
}

pub fn security_actions_for_mapper_event(
    event: SecurityMapperRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    let security = |action| Action::Security(action);
    match event {
        SecurityMapperRunnerEvent::Started { id } => {
            vec![security(SecurityAction::SessionRunning(id))]
        }
        SecurityMapperRunnerEvent::Output {
            id,
            stream,
            line,
            truncated,
        } => vec![security(SecurityAction::SessionOutput {
            id,
            stream,
            line,
            truncated,
        })],
        SecurityMapperRunnerEvent::Completed { id, .. } => {
            vec![security(SecurityAction::CompleteSession {
                id,
                result_paths: Vec::new(),
                finished_at: timestamp,
            })]
        }
        SecurityMapperRunnerEvent::Failed { id, exit_code } => {
            vec![security(SecurityAction::FailSession {
                id,
                message: exit_code.map_or_else(
                    || "Security package mapping failed without an exit code".into(),
                    |code| format!("Security package mapping failed with exit code {code}"),
                ),
                finished_at: timestamp,
            })]
        }
        SecurityMapperRunnerEvent::CancellationRequested { .. } => Vec::new(),
        SecurityMapperRunnerEvent::Cancelled {
            id,
            forced,
            exit_code: _,
        } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(security(SecurityAction::SessionOutput {
                    id,
                    stream: SecurityOutputStream::Stderr,
                    line: "Security package mapping required forced termination".into(),
                    truncated: false,
                }));
            }
            actions.push(security(SecurityAction::CancelSession {
                id,
                finished_at: timestamp,
            }));
            actions
        }
        SecurityMapperRunnerEvent::CancellationRejected { id, message } => {
            vec![security(SecurityAction::RejectCancellation { id, message })]
        }
        SecurityMapperRunnerEvent::TimedOut { id, forced, .. } => {
            let mode = if forced { "forced" } else { "graceful" };
            vec![
                security(SecurityAction::SessionOutput {
                    id,
                    stream: SecurityOutputStream::Stderr,
                    line: format!(
                        "Security package mapping timed out; {mode} termination was used"
                    ),
                    truncated: false,
                }),
                security(SecurityAction::TimeoutSession {
                    id,
                    finished_at: timestamp,
                }),
            ]
        }
        SecurityMapperRunnerEvent::Lost { id, message } => {
            vec![security(SecurityAction::LoseSession {
                id,
                message,
                finished_at: timestamp,
            })]
        }
    }
}

pub fn sdk_actions_for_runner_event(
    id: SdkSessionId,
    event: SdkToolRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        SdkToolRunnerEvent::Started => vec![
            Action::SdkSessionStarting {
                id,
                started_at: timestamp,
            },
            Action::SdkSessionRunning { id },
        ],
        SdkToolRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendSdkSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        }],
        SdkToolRunnerEvent::Completed { exit_code } => vec![Action::CompleteSdkSession {
            id,
            exit_code: exit_code.unwrap_or(0),
            artifacts: Vec::new(),
            finished_at: timestamp,
        }],
        SdkToolRunnerEvent::Failed { exit_code } => vec![Action::FailSdkSession {
            id,
            message: exit_code.map_or_else(
                || "SDK tool exited unsuccessfully without an exit code".into(),
                |code| format!("SDK tool exited unsuccessfully with exit code {code}"),
            ),
            exit_code,
            finished_at: timestamp,
        }],
        SdkToolRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendSdkSessionOutput {
                    id,
                    stream: SdkOutputStream::Stderr,
                    line: "SDK tool cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelSdkSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        SdkToolRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectSdkSessionCancellation { id, message }]
        }
        SdkToolRunnerEvent::TimedOut { forced, exit_code } => {
            let mode = if forced { "forced" } else { "graceful" };
            vec![Action::FailSdkSession {
                id,
                message: format!("SDK tool timed out; {mode} termination was used"),
                exit_code,
                finished_at: timestamp,
            }]
        }
        SdkToolRunnerEvent::Lost { message } => vec![Action::LoseSdkSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}

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

pub fn wic_capability_action(capability: WicCapability) -> Action {
    Action::WicCapabilityLoaded(capability)
}

pub fn wic_device_inventory_action(response: WicDeviceInventoryResponse) -> Action {
    Action::WicDeviceInventoryLoaded {
        request: response.request,
        devices: response.devices,
        limitations: response.limitations,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WicSessionEvent {
    Starting,
    Started,
    Output {
        stream: WicOutputStream,
        line: String,
        truncated: bool,
    },
    Completed {
        exit_code: i32,
        outputs: Vec<WicOutput>,
        limitations: Vec<String>,
    },
    Failed {
        message: String,
        exit_code: Option<i32>,
    },
    Cancelled {
        forced: bool,
        exit_code: Option<i32>,
    },
    CancellationRejected {
        message: String,
    },
    Lost {
        message: String,
    },
}

pub fn wic_actions_for_session_event(
    id: WicSessionId,
    event: WicSessionEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        WicSessionEvent::Starting => vec![Action::WicSessionStarting {
            id,
            started_at: timestamp,
        }],
        WicSessionEvent::Started => vec![Action::WicSessionRunning { id }],
        WicSessionEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendWicSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        }],
        WicSessionEvent::Completed {
            exit_code,
            outputs,
            limitations,
        } => vec![Action::CompleteWicSession {
            id,
            exit_code,
            outputs,
            limitations,
            finished_at: timestamp,
        }],
        WicSessionEvent::Failed { message, exit_code } => vec![Action::FailWicSession {
            id,
            message,
            exit_code,
            finished_at: timestamp,
        }],
        WicSessionEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendWicSessionOutput {
                    id,
                    stream: WicOutputStream::Stderr,
                    line: "Wic cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelWicSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        WicSessionEvent::CancellationRejected { message } => {
            vec![Action::RejectWicSessionCancellation { id, message }]
        }
        WicSessionEvent::Lost { message } => vec![Action::LoseWicSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}

pub fn wic_actions_for_runner_event(
    id: WicSessionId,
    event: WicRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    let event = match event {
        WicRunnerEvent::Starting => WicSessionEvent::Starting,
        WicRunnerEvent::Started => WicSessionEvent::Started,
        WicRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => WicSessionEvent::Output {
            stream: match stream {
                WicRunnerOutputStream::Stdout => WicOutputStream::Stdout,
                WicRunnerOutputStream::Stderr => WicOutputStream::Stderr,
            },
            line,
            truncated,
        },
        WicRunnerEvent::Completed {
            exit_code,
            outputs,
            limitations,
        } => WicSessionEvent::Completed {
            exit_code,
            outputs,
            limitations,
        },
        WicRunnerEvent::Failed { message, exit_code } => {
            WicSessionEvent::Failed { message, exit_code }
        }
        WicRunnerEvent::Cancelled { forced, exit_code } => {
            WicSessionEvent::Cancelled { forced, exit_code }
        }
        WicRunnerEvent::CancellationRejected { message } => {
            WicSessionEvent::CancellationRejected { message }
        }
        WicRunnerEvent::Lost { message } => WicSessionEvent::Lost { message },
    };
    wic_actions_for_session_event(id, event, timestamp)
}

pub fn qemu_actions_for_runner_event(
    id: QemuSessionId,
    event: QemuRunnerEvent,
    timestamp: SystemTime,
) -> Vec<Action> {
    match event {
        QemuRunnerEvent::Starting => vec![Action::QemuSessionStarting {
            id,
            started_at: timestamp,
        }],
        QemuRunnerEvent::Started => vec![Action::QemuSessionRunning { id }],
        QemuRunnerEvent::Output {
            stream,
            line,
            truncated,
        } => vec![Action::AppendQemuSessionOutput {
            id,
            stream: match stream {
                QemuRunnerOutputStream::Stdout => QemuOutputStream::Stdout,
                QemuRunnerOutputStream::Stderr => QemuOutputStream::Stderr,
            },
            line,
            truncated,
            timestamp,
        }],
        QemuRunnerEvent::Completed { exit_code } => vec![Action::CompleteQemuSession {
            id,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Failed { message, exit_code } => vec![Action::FailQemuSession {
            id,
            message,
            exit_code,
            finished_at: timestamp,
        }],
        QemuRunnerEvent::Cancelled { forced, exit_code } => {
            let mut actions = Vec::new();
            if forced {
                actions.push(Action::AppendQemuSessionOutput {
                    id,
                    stream: QemuOutputStream::Stderr,
                    line: "runqemu cancellation required forced termination".into(),
                    truncated: false,
                    timestamp,
                });
            }
            actions.push(Action::CancelQemuSession {
                id,
                exit_code,
                finished_at: timestamp,
            });
            actions
        }
        QemuRunnerEvent::CancellationRejected { message } => {
            vec![Action::RejectQemuSessionCancellation { id, message }]
        }
        QemuRunnerEvent::Lost { message } => vec![Action::LoseQemuSession {
            id,
            message,
            finished_at: timestamp,
        }],
    }
}
