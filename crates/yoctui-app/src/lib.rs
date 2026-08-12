//! Application-owned input mapping, keeping terminal concerns outside the reducer.
use std::time::SystemTime;
use yoctui_bitbake::{
    BackendEvent, DevtoolOutputStream, DevtoolRunnerEvent, QaLayerCapabilityResponse,
    QaLayerRunnerEvent, QaReportAdapterError, QaReportResponse, QaReportScanOutcome,
    QaTaskCapabilityResponse, QemuRunnerEvent, QemuRunnerOutputStream, SdkToolRunnerEvent,
    SecurityMapperRunnerEvent, TestResultImportResponse, TestResultOperation,
    TestResultRunnerEvent, TestRunnerEvent, WicDeviceInventoryResponse, WicRunnerEvent,
    WicRunnerOutputStream,
};
use yoctui_model::{
    Action, AppError, BackgroundJobContext, BackgroundJobError, BackgroundJobId, BackgroundJobKind,
    BackgroundJobOutputEntry, BackgroundJobOutputSource, BackgroundJobProgress,
    BackgroundJobResult, BackgroundJobSpec, BuildRequest, DevtoolOperation, FocusTarget,
    LayerInspectorMode, LayerRelationship, LayerRelationships, MaintenanceAction,
    MaintenanceBuildHistoryField, MaintenanceCleanupField, MaintenanceDialog,
    MaintenanceGitArchiveField, MaintenanceLockedCacheField, MaintenanceReadinessField,
    MaintenanceView, PopupEditorCommand, QaAction, QaDialog, QaReportFailureKind, QaReportRequest,
    QaView, QemuOutputStream, QemuSessionId, RecipeDependencies, Screen, SdkBuildAction, SdkKind,
    SdkOutputStream, SdkSessionId, SecurityAction, SecurityDialog, SecurityOutputStream,
    SecurityView, Severity, TaskId, TaskInfo, TestComparison, VariableDetail, VariableIdentity,
    WicCapability, WicOutput, WicOutputStream, WicSessionId,
};

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

fn test_result_failed_action(operation: TestResultOperation, message: String) -> Vec<Action> {
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

#[derive(Debug)]
pub struct BuildJobCoordinator {
    next_job_id: u64,
    active_job: Option<BackgroundJobId>,
    active_kind: Option<BackgroundJobKind>,
    cancellation_requested: bool,
}
impl Default for BuildJobCoordinator {
    fn default() -> Self {
        Self {
            next_job_id: 1,
            active_job: None,
            active_kind: None,
            cancellation_requested: false,
        }
    }
}
impl BuildJobCoordinator {
    pub fn active_job_id(&self) -> Option<BackgroundJobId> {
        self.active_job
    }

    pub fn queue_build(
        &mut self,
        request: &BuildRequest,
        queued_at: SystemTime,
    ) -> Option<Vec<Action>> {
        if self.active_job.is_some() || request.validate().is_err() {
            return None;
        }
        let id = BackgroundJobId(self.next_job_id);
        self.next_job_id = self.next_job_id.checked_add(1).unwrap_or(1);
        self.active_job = Some(id);
        self.cancellation_requested = false;
        let target = request.targets.first().cloned();
        let (kind, title, workspace, recipe) = match request.task.as_deref() {
            Some("cve_check") => (
                BackgroundJobKind::CveCheck,
                format!("CVE check {}", request.targets.join(" ")),
                Screen::Recipes,
                target.clone(),
            ),
            Some("create_spdx") => (
                BackgroundJobKind::Spdx,
                format!("SPDX generation {}", request.targets.join(" ")),
                Screen::Recipes,
                target.clone(),
            ),
            Some(task @ ("testimage" | "testsdk" | "testsdkext")) => (
                BackgroundJobKind::Test,
                format!("Test {}:{task}", request.targets.join(" ")),
                Screen::Testing,
                None,
            ),
            Some(task) => (
                BackgroundJobKind::Build,
                format!("Build {}:{task}", request.targets.join(" ")),
                Screen::Tasks,
                None,
            ),
            None => (
                BackgroundJobKind::Build,
                format!("Build {}", request.targets.join(" ")),
                Screen::Tasks,
                None,
            ),
        };
        self.active_kind = Some(kind);
        Some(vec![
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind,
                title,
                context: BackgroundJobContext {
                    workspace: Some(workspace),
                    target,
                    recipe,
                    task: request.task.clone(),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at,
            }),
            Action::StartBackgroundJob {
                id,
                started_at: queued_at,
            },
        ])
    }

    pub fn start_failed(&mut self, message: String, finished_at: SystemTime) -> Vec<Action> {
        self.active_job.take().map_or_else(Vec::new, |id| {
            self.active_kind = None;
            self.cancellation_requested = false;
            vec![Action::FailBackgroundJob {
                id,
                error: BackgroundJobError {
                    summary: "could not start BitBake".into(),
                    detail: Some(message),
                },
                finished_at,
            }]
        })
    }

    pub fn request_cancellation(&mut self) -> Option<Action> {
        let id = self.active_job?;
        if self.cancellation_requested {
            return None;
        }
        self.cancellation_requested = true;
        Some(Action::RequestBackgroundJobCancellation { id })
    }

    pub fn cancellation_failed(&mut self, message: String, timestamp: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        self.cancellation_requested = false;
        vec![
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Error,
                    message: format!("Cancellation request failed: {message}"),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp,
                },
            },
            Action::RejectBackgroundJobCancellation { id },
            Action::BuildCancellationRejected(message),
        ]
    }

    pub fn backend_lost(&mut self, message: String, timestamp: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job.take() else {
            return Vec::new();
        };
        self.active_kind = None;
        self.cancellation_requested = false;
        vec![
            Action::Failure(AppError::new(
                "Backend",
                message.clone(),
                "inspect backend diagnostics and restart the build",
            )),
            Action::LoseBackgroundJob {
                id,
                error: BackgroundJobError {
                    summary: "BitBake backend lost".into(),
                    detail: Some(message),
                },
                finished_at: timestamp,
            },
        ]
    }

    pub fn job_actions_for_event(
        &mut self,
        event: &BackendEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        match event {
            BackendEvent::BuildStarted => vec![Action::RunBackgroundJob { id }],
            BackendEvent::ParseProgress {
                current: Some(completed),
                total: Some(total),
            } if *total > 0 && completed <= total => {
                vec![Action::UpdateBackgroundJobProgress {
                    id,
                    progress: BackgroundJobProgress::Units {
                        completed: *completed,
                        total: *total,
                    },
                }]
            }
            BackendEvent::Log(entry) => vec![Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: entry.severity,
                    message: entry.message.clone(),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp: entry.timestamp,
                },
            }],
            BackendEvent::BuildCompleted { success, exit_code } => {
                self.active_job = None;
                let kind = self.active_kind.take().unwrap_or(BackgroundJobKind::Build);
                let cancellation_requested = self.cancellation_requested;
                self.cancellation_requested = false;
                if cancellation_requested && !success {
                    vec![Action::CancelBackgroundJob {
                        id,
                        finished_at: timestamp,
                    }]
                } else if *success {
                    vec![Action::SucceedBackgroundJob {
                        id,
                        result: BackgroundJobResult {
                            summary: match kind {
                                BackgroundJobKind::CveCheck => {
                                    "CVE check completed; BitBake reported no result path".into()
                                }
                                BackgroundJobKind::Spdx => {
                                    "SPDX generation completed; BitBake reported no result path"
                                        .into()
                                }
                                _ => "BitBake build completed successfully".into(),
                            },
                            artifacts: Vec::new(),
                        },
                        finished_at: timestamp,
                    }]
                } else {
                    vec![Action::FailBackgroundJob {
                        id,
                        error: BackgroundJobError {
                            summary: "BitBake build failed".into(),
                            detail: exit_code.map(|code| format!("exit code {code}")),
                        },
                        finished_at: timestamp,
                    }]
                }
            }
            BackendEvent::CommandFailed { code, message } => {
                self.active_job = None;
                self.active_kind = None;
                self.cancellation_requested = false;
                vec![Action::FailBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: format!("BitBake command failed: {code}"),
                        detail: Some(message.clone()),
                    },
                    finished_at: timestamp,
                }]
            }
            BackendEvent::Disconnected => {
                self.active_job = None;
                self.active_kind = None;
                self.cancellation_requested = false;
                vec![Action::LoseBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "BitBake backend disconnected".into(),
                        detail: None,
                    },
                    finished_at: timestamp,
                }]
            }
            BackendEvent::Workspace(_)
            | BackendEvent::Recipes(_)
            | BackendEvent::Layers(_)
            | BackendEvent::Variable { .. }
            | BackendEvent::Dependencies { .. }
            | BackendEvent::DependencyGraph { .. }
            | BackendEvent::DependencyGraphFailed { .. }
            | BackendEvent::SignatureDump { .. }
            | BackendEvent::SignatureDumpFailed { .. }
            | BackendEvent::SignatureComparison { .. }
            | BackendEvent::SignatureComparisonFailed { .. }
            | BackendEvent::PackageInventory { .. }
            | BackendEvent::PackageInventoryFailed { .. }
            | BackendEvent::PackageDetail { .. }
            | BackendEvent::PackageDetailFailed { .. }
            | BackendEvent::ImageArtifacts { .. }
            | BackendEvent::ImageArtifactsFailed { .. }
            | BackendEvent::RecipeSources { .. }
            | BackendEvent::RecipeMetadata(_)
            | BackendEvent::LayerRelationships(_)
            | BackendEvent::ParseProgress { .. }
            | BackendEvent::TaskQueued { .. }
            | BackendEvent::TaskStarted { .. }
            | BackendEvent::TaskProgress { .. }
            | BackendEvent::TaskCompleted { .. }
            | BackendEvent::Ignored => Vec::new(),
        }
    }

    pub fn actions_for_backend_event(
        &mut self,
        event: BackendEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let cancellation_acknowledged = self.cancellation_requested
            && matches!(&event, BackendEvent::BuildCompleted { success: false, .. });
        let mut actions = if cancellation_acknowledged {
            let exit_code = match &event {
                BackendEvent::BuildCompleted { exit_code, .. } => *exit_code,
                _ => None,
            };
            vec![Action::BuildCancelled { exit_code }]
        } else {
            model_action_from_backend_event(event.clone())
                .into_iter()
                .collect()
        };
        actions.extend(self.job_actions_for_event(&event, timestamp));
        actions
    }
}

#[derive(Debug)]
pub struct DevtoolJobCoordinator {
    next_job_id: u64,
    active_job: Option<BackgroundJobId>,
    active_operation: Option<DevtoolOperation>,
    cancellation_requested: bool,
}
impl Default for DevtoolJobCoordinator {
    fn default() -> Self {
        Self {
            next_job_id: 1_u64 << 63,
            active_job: None,
            active_operation: None,
            cancellation_requested: false,
        }
    }
}
impl DevtoolJobCoordinator {
    pub fn active_job_id(&self) -> Option<BackgroundJobId> {
        self.active_job
    }

    pub fn active_operation(&self) -> Option<&DevtoolOperation> {
        self.active_operation.as_ref()
    }

    pub fn queue(
        &mut self,
        operation: DevtoolOperation,
        queued_at: SystemTime,
    ) -> Option<Vec<Action>> {
        if self.active_job.is_some() || operation.validate().is_err() {
            return None;
        }
        let id = BackgroundJobId(self.next_job_id);
        self.next_job_id = self.next_job_id.checked_add(1).unwrap_or(1_u64 << 63);
        let recipe = operation.recipe().to_owned();
        let (label, target, path) = match &operation {
            DevtoolOperation::Modify { .. } => ("modify", None, None),
            DevtoolOperation::UpdateRecipe { .. } => ("update-recipe", None, None),
            DevtoolOperation::Finish { destination, .. } => {
                ("finish", None, Some(destination.clone()))
            }
            DevtoolOperation::DeployTarget { target, .. } => {
                ("deploy-target", Some(target.clone()), None)
            }
            DevtoolOperation::UndeployTarget { target, .. } => {
                ("undeploy-target", Some(target.clone()), None)
            }
            DevtoolOperation::Reset { .. } => ("reset", None, None),
        };
        self.active_job = Some(id);
        self.active_operation = Some(operation);
        self.cancellation_requested = false;
        Some(vec![
            Action::QueueBackgroundJob(BackgroundJobSpec {
                id,
                kind: BackgroundJobKind::Devtool,
                title: format!("Devtool {label} {recipe}"),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Recipes),
                    target,
                    recipe: Some(recipe),
                    path,
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at,
            }),
            Action::StartBackgroundJob {
                id,
                started_at: queued_at,
            },
        ])
    }

    pub fn start_failed(&mut self, message: String, finished_at: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job.take() else {
            return Vec::new();
        };
        self.active_operation = None;
        self.cancellation_requested = false;
        vec![Action::FailBackgroundJob {
            id,
            error: BackgroundJobError {
                summary: "Could not start Devtool".into(),
                detail: Some(message),
            },
            finished_at,
        }]
    }

    pub fn request_cancellation(&mut self) -> Option<Action> {
        let id = self.active_job?;
        if self.cancellation_requested {
            return None;
        }
        self.cancellation_requested = true;
        Some(Action::RequestBackgroundJobCancellation { id })
    }

    pub fn cancellation_failed(&mut self, message: String, timestamp: SystemTime) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        self.cancellation_requested = false;
        vec![
            Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Error,
                    message: format!("Devtool cancellation failed: {message}"),
                    source: BackgroundJobOutputSource::Backend,
                    truncated: false,
                    timestamp,
                },
            },
            Action::RejectBackgroundJobCancellation { id },
        ]
    }

    pub fn actions_for_event(
        &mut self,
        event: DevtoolRunnerEvent,
        timestamp: SystemTime,
    ) -> Vec<Action> {
        let Some(id) = self.active_job else {
            return Vec::new();
        };
        match event {
            DevtoolRunnerEvent::Started => vec![Action::RunBackgroundJob { id }],
            DevtoolRunnerEvent::Output {
                stream,
                line,
                truncated,
            } => vec![Action::AppendBackgroundJobOutput {
                id,
                entry: BackgroundJobOutputEntry {
                    severity: Severity::Info,
                    message: line,
                    source: match stream {
                        DevtoolOutputStream::Stdout => BackgroundJobOutputSource::Stdout,
                        DevtoolOutputStream::Stderr => BackgroundJobOutputSource::Stderr,
                    },
                    truncated,
                    timestamp,
                },
            }],
            DevtoolRunnerEvent::Completed { exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::SucceedBackgroundJob {
                    id,
                    result: BackgroundJobResult {
                        summary: exit_code.map_or_else(
                            || "Devtool completed successfully".into(),
                            |code| format!("Devtool completed successfully (exit code {code})"),
                        ),
                        artifacts: Vec::new(),
                    },
                    finished_at: timestamp,
                }]
            }
            DevtoolRunnerEvent::Failed { exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::FailBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "Devtool failed".into(),
                        detail: exit_code.map(|code| format!("exit code {code}")),
                    },
                    finished_at: timestamp,
                }]
            }
            DevtoolRunnerEvent::Cancelled { forced, exit_code } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                let mut actions = Vec::new();
                if forced {
                    actions.push(Action::AppendBackgroundJobOutput {
                        id,
                        entry: BackgroundJobOutputEntry {
                            severity: Severity::Warning,
                            message: "Devtool cancellation required forced termination".into(),
                            source: BackgroundJobOutputSource::Backend,
                            truncated: false,
                            timestamp,
                        },
                    });
                }
                if let Some(code) = exit_code {
                    actions.push(Action::AppendBackgroundJobOutput {
                        id,
                        entry: BackgroundJobOutputEntry {
                            severity: Severity::Info,
                            message: format!("Devtool cancellation exit code {code}"),
                            source: BackgroundJobOutputSource::Backend,
                            truncated: false,
                            timestamp,
                        },
                    });
                }
                actions.push(Action::CancelBackgroundJob {
                    id,
                    finished_at: timestamp,
                });
                actions
            }
            DevtoolRunnerEvent::Lost { message } => {
                self.active_job = None;
                self.active_operation = None;
                self.cancellation_requested = false;
                vec![Action::LoseBackgroundJob {
                    id,
                    error: BackgroundJobError {
                        summary: "Devtool process lost".into(),
                        detail: Some(message),
                    },
                    finished_at: timestamp,
                }]
            }
        }
    }
}

pub fn model_action_from_backend_event(event: BackendEvent) -> Option<Action> {
    match event {
        BackendEvent::Workspace(workspace) => Some(Action::WorkspaceLoaded(workspace)),
        BackendEvent::BuildStarted => Some(Action::BuildStarted),
        BackendEvent::ParseProgress { current, total } => {
            Some(Action::ParseProgress { current, total })
        }
        BackendEvent::Log(entry) => Some(Action::Log(entry)),
        BackendEvent::TaskQueued {
            recipe,
            task,
            worker,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.worker = worker;
            info.stats = stats;
            Some(Action::TaskQueued(info))
        }
        BackendEvent::TaskStarted {
            recipe,
            task,
            pid,
            worker,
            log_path,
            stats,
        } => {
            let id = TaskId(format!("{recipe}:{task}"));
            let mut info = TaskInfo::active(id, recipe, task);
            info.pid = pid;
            info.worker = worker;
            info.log_path = log_path;
            info.stats = stats;
            Some(Action::TaskStarted(info))
        }
        BackendEvent::TaskProgress {
            recipe,
            task,
            progress,
        } => Some(Action::TaskProgress {
            id: TaskId(format!("{recipe}:{task}")),
            progress,
        }),
        BackendEvent::TaskCompleted {
            recipe,
            task,
            success,
        } => Some(Action::TaskCompleted {
            id: TaskId(format!("{recipe}:{task}")),
            success,
        }),
        BackendEvent::BuildCompleted { success, exit_code } => {
            Some(Action::BuildCompleted { success, exit_code })
        }
        BackendEvent::CommandFailed { code, message } => Some(Action::Failure(AppError::new(
            "BitBake",
            format!("{code}: {message}"),
            "inspect the bridge or BitBake diagnostics",
        ))),
        BackendEvent::Disconnected => Some(Action::Failure(AppError::new(
            "Bridge",
            "backend disconnected",
            "restart Yoctui and inspect the backend diagnostics",
        ))),
        BackendEvent::Recipes(recipes) => Some(Action::RecipesLoaded(recipes)),
        BackendEvent::Layers(layers) => Some(Action::LayersLoaded(layers)),
        BackendEvent::Variable {
            name,
            recipe,
            value,
            provenance,
            unexpanded_value,
            operations,
            active_overrides,
        } => Some(Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity { name, recipe },
            effective_value: value,
            unexpanded_value,
            provenance,
            operations,
            active_overrides,
        })),
        BackendEvent::Dependencies {
            recipe,
            build,
            runtime,
        } => Some(Action::DependenciesLoaded(RecipeDependencies {
            recipe,
            build,
            runtime,
        })),
        BackendEvent::DependencyGraph { graph, limitations } => {
            if limitations.is_empty() {
                Some(Action::DependencyGraphLoaded(graph))
            } else {
                Some(Action::DependencyGraphPartial { graph, limitations })
            }
        }
        BackendEvent::DependencyGraphFailed { root, message } => {
            Some(Action::DependencyGraphFailed { root, message })
        }
        BackendEvent::SignatureDump {
            target,
            records,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureDumpLoaded { target, records })
            } else {
                Some(Action::SignatureDumpPartial {
                    target,
                    records,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureDumpFailed { target, message } => {
            Some(Action::SignatureDumpFailed { target, message })
        }
        BackendEvent::SignatureComparison {
            request,
            differences,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::SignatureComparisonLoaded {
                    request,
                    differences,
                })
            } else {
                Some(Action::SignatureComparisonPartial {
                    request,
                    differences,
                    limitations,
                })
            }
        }
        BackendEvent::SignatureComparisonFailed { request, message } => {
            Some(Action::SignatureComparisonFailed { request, message })
        }
        BackendEvent::PackageInventory {
            request,
            packages,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageInventoryLoaded { request, packages })
            } else {
                Some(Action::PackageInventoryPartial {
                    request,
                    packages,
                    limitations,
                })
            }
        }
        BackendEvent::PackageInventoryFailed { request, message } => {
            Some(Action::PackageInventoryFailed { request, message })
        }
        BackendEvent::PackageDetail {
            request,
            detail,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::PackageDetailLoaded { request, detail })
            } else {
                Some(Action::PackageDetailPartial {
                    request,
                    detail,
                    limitations,
                })
            }
        }
        BackendEvent::PackageDetailFailed { request, message } => {
            Some(Action::PackageDetailFailed { request, message })
        }
        BackendEvent::ImageArtifacts {
            request,
            inventory,
            limitations,
        } => {
            if limitations.is_empty() {
                Some(Action::ImageArtifactInventoryLoaded { request, inventory })
            } else {
                Some(Action::ImageArtifactInventoryPartial {
                    request,
                    inventory,
                    limitations,
                })
            }
        }
        BackendEvent::ImageArtifactsFailed { request, message } => {
            Some(Action::ImageArtifactInventoryFailed { request, message })
        }
        BackendEvent::RecipeSources { recipe, paths } => {
            Some(Action::RecipeSourcesLoaded { recipe, paths })
        }
        BackendEvent::RecipeMetadata(metadata) => Some(Action::RecipeMetadataLoaded(metadata)),
        BackendEvent::LayerRelationships(layers) => {
            Some(Action::LayerRelationshipsLoaded(LayerRelationships {
                layers: layers
                    .into_iter()
                    .map(|layer| LayerRelationship {
                        name: layer.name,
                        priority: layer.priority,
                        compatible: layer.compatible,
                        depends: layer.depends,
                        overlays: layer.overlays,
                        appends: layer.appends,
                    })
                    .collect(),
            }))
        }
        BackendEvent::Ignored => None,
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Esc,
    Enter,
    CtrlC,
    CtrlV,
    CtrlB,
    CtrlP,
    F5,
    Tab,
    BackTab,
    CtrlS,
    Up,
    Down,
    Backspace,
    Left,
    Right,
    Home,
    End,
}

pub fn popup_editor_action(editing: bool, key: Input) -> Option<Action> {
    let command = if editing {
        match key {
            Input::Esc => PopupEditorCommand::ToggleInsert,
            Input::Backspace => PopupEditorCommand::Backspace,
            Input::Left => PopupEditorCommand::Left,
            Input::Right => PopupEditorCommand::Right,
            Input::Up => PopupEditorCommand::Up,
            Input::Down => PopupEditorCommand::Down,
            Input::Home => PopupEditorCommand::Home,
            Input::End => PopupEditorCommand::End,
            Input::CtrlC => PopupEditorCommand::Copy,
            Input::CtrlV => PopupEditorCommand::Paste,
            Input::Char(character) => PopupEditorCommand::Insert(character),
            _ => return None,
        }
    } else {
        match key {
            Input::Char('i') => PopupEditorCommand::ToggleInsert,
            Input::Char('e') => PopupEditorCommand::SelectValue,
            Input::Left | Input::Char('h') => PopupEditorCommand::Left,
            Input::Right | Input::Char('l') => PopupEditorCommand::Right,
            Input::Up | Input::Char('k') => PopupEditorCommand::Up,
            Input::Down | Input::Char('j') => PopupEditorCommand::Down,
            Input::Home => PopupEditorCommand::Home,
            Input::End => PopupEditorCommand::End,
            Input::CtrlC => PopupEditorCommand::Copy,
            _ => return None,
        }
    };
    Some(Action::EditActivePopup(command))
}
pub fn key_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('b') => None,
        Input::Char('c') => Some(Action::Cancel),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Backspace => Some(Action::BackspaceLogQuery),
        Input::Up => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('l') => Some(Action::Open(Screen::Logs)),
        Input::Char('h') => Some(Action::Open(Screen::BuildHistory)),
        Input::Char('e') => Some(Action::Open(Screen::Errors)),
        Input::Char('r') => Some(Action::Open(Screen::Recipes)),
        Input::Char('y') => Some(Action::Open(Screen::Layers)),
        Input::Char('v') => Some(Action::Open(Screen::Configuration)),
        Input::Char('x') => Some(Action::Open(Screen::Bbmask)),
        Input::Char('?') => Some(Action::Open(Screen::Help)),
        Input::Char('q') | Input::CtrlC => Some(Action::Quit),
        Input::CtrlP => Some(Action::OpenCommandPalette),
        Input::F5 => Some(Action::OpenBuildOptions),
        Input::Tab => Some(Action::CycleFocus { backwards: false }),
        Input::BackTab => Some(Action::CycleFocus { backwards: true }),
        Input::Char('Y') => Some(Action::ConfirmQuit),
        Input::Enter => Some(Action::ActivateNotification),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}

pub fn focus_action(focus: FocusTarget, key: Input) -> Option<Action> {
    match (focus, key) {
        (FocusTarget::Navigator, Input::Up | Input::Char('k')) => {
            Some(Action::SelectNavigator { delta: -1 })
        }
        (FocusTarget::Navigator, Input::Down | Input::Char('j')) => {
            Some(Action::SelectNavigator { delta: 1 })
        }
        (FocusTarget::Navigator, Input::Enter) => Some(Action::ActivateNavigator),
        (FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector, Input::Tab) => {
            Some(Action::CycleFocus { backwards: false })
        }
        (
            FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector,
            Input::BackTab,
        ) => Some(Action::CycleFocus { backwards: true }),
        (FocusTarget::Navigator | FocusTarget::Inspector, Input::Esc) => {
            Some(Action::Focus(FocusTarget::Workspace))
        }
        _ => None,
    }
}

pub fn settings_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSetting { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSetting { delta: 1 }),
        Input::Left => Some(Action::ChangeSelectedSetting { backwards: true }),
        Input::Right | Input::Enter => Some(Action::ChangeSelectedSetting { backwards: false }),
        Input::Char('r') => Some(Action::RetrySettingsPersistence),
        _ => None,
    }
}

pub fn build_environment_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('e') => Some(Action::OpenBuildEnvironmentEditor),
        Input::Char('c') => Some(Action::OpenBuildEnvironmentCloneEditor),
        Input::Up | Input::Char('k') => Some(Action::SelectBuildEnvironmentField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectBuildEnvironmentField { delta: 1 }),
        Input::Char('s') => Some(Action::ApplyBuildEnvironmentProfile),
        Input::Char('V') => Some(Action::BeginBuildEnvironmentVerification),
        Input::Esc => Some(Action::Open(Screen::Dashboard)),
        _ => None,
    }
}
pub fn tasks_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendTaskFilter(character)),
            Input::Backspace => Some(Action::BackspaceTaskFilter),
            Input::Enter | Input::Esc => Some(Action::FinishTaskFilterEdit),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::ScrollBuildTasks { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollBuildTasks { delta: 1 }),
        Input::Char('f') => Some(Action::CycleTaskStateFilter),
        Input::Char('F') => Some(Action::CycleTaskFilterField),
        Input::Char('/') => Some(Action::BeginTaskFilterEdit),
        Input::Char('d') => Some(Action::CycleTaskDurationFilter),
        _ => None,
    }
}
pub fn logs_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendLogQuery(character)),
            Input::Backspace => Some(Action::BackspaceLogQuery),
            Input::Enter | Input::Esc => Some(Action::FinishLogSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::ScrollLogs { delta: 1 }),
        Input::Down | Input::Char('j') => Some(Action::ScrollLogs { delta: -1 }),
        Input::Left => Some(Action::ScrollLogsHorizontally { delta: -8 }),
        Input::Right => Some(Action::ScrollLogsHorizontally { delta: 8 }),
        Input::Char('f') => Some(Action::ToggleLogFollow),
        Input::Char('w') => Some(Action::ToggleLogWrap),
        Input::Char('s') => Some(Action::CycleLogSeverity),
        Input::Char('/') => Some(Action::BeginLogSearch),
        Input::Char('n') => Some(Action::NextLogMatch),
        Input::Char('N') => Some(Action::PreviousLogMatch),
        Input::Char('R') => Some(Action::CycleLogRecipeFilter),
        Input::Char('T') => Some(Action::CycleLogTaskFilter),
        Input::Char('B') => Some(Action::CycleLogBuildFilter),
        Input::Char('o') => Some(Action::OpenSelectedLogSource),
        Input::Char('C') => Some(Action::CopySelectedLog),
        _ => None,
    }
}
pub fn errors_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectError { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectError { delta: 1 }),
        Input::Enter => Some(Action::JumpToSelectedError),
        Input::Char('o') => Some(Action::OpenSelectedErrorSource),
        _ => None,
    }
}
pub fn dependency_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDependencyGraphNode { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDependencyGraphNode { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedDependencyRecipe),
        Input::Char('o') => Some(Action::OpenSelectedDependencyProvider),
        Input::Char('L') => Some(Action::OpenSelectedDependencyTaskLog),
        Input::Char('r') => Some(Action::RefreshDependencyGraph),
        _ => None,
    }
}
pub fn signature_task_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureTask { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureTask { delta: 1 }),
        Input::Enter => Some(Action::ConfirmSignatureTask),
        Input::Esc => Some(Action::CancelSignatureTaskPicker),
        _ => None,
    }
}
pub fn signature_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSignatureRecord { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSignatureRecord { delta: 1 }),
        Input::Char('1') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Left,
        )),
        Input::Char('2') => Some(Action::SetSelectedSignatureComparisonSide(
            yoctui_model::SignatureComparisonSide::Right,
        )),
        Input::Char('c') => Some(Action::BeginSignatureComparison),
        Input::Char('r') => Some(Action::RefreshSignatureDump),
        Input::Char('e') => Some(Action::OpenSignatureProvider),
        Input::Esc => Some(Action::LeaveSignatureWorkspace),
        _ => None,
    }
}
pub fn package_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendPackageQuery(character)),
            Input::Backspace => Some(Action::BackspacePackageQuery),
            Input::Enter | Input::Esc => Some(Action::FinishPackageSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectPackage { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectPackage { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedPackageDetail),
        Input::Char('/') => Some(Action::BeginPackageSearch),
        Input::Char('R') => Some(Action::RefreshPackageInventory),
        Input::Char('c') => Some(Action::CancelPackageOperation),
        Input::Char('D') => Some(Action::TogglePackageDependencyKind),
        Input::Char('[') => Some(Action::SelectPackageDependency { delta: -1 }),
        Input::Char(']') => Some(Action::SelectPackageDependency { delta: 1 }),
        Input::Char('d') => Some(Action::OpenSelectedPackageDependency),
        Input::Char('u') => Some(Action::BackPackageNavigation),
        Input::Char('o') => Some(Action::OpenSelectedPackageRecipe),
        Input::Char('e') => Some(Action::OpenSelectedPackageProvider),
        _ => None,
    }
}

pub fn images_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendImageArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceImageArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishImageArtifactSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectImageArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectImageArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginImageArtifactSearch),
        Input::Char('R') => Some(Action::RefreshImageArtifactInventory),
        Input::Char('c') => Some(Action::CancelImageArtifactOperation),
        Input::Char('b') => Some(Action::BeginSelectedImageArtifactBuild),
        Input::Char('Q') => Some(Action::BeginSelectedQemuLaunch),
        Input::Char('W') => Some(Action::BeginSelectedWicCreate),
        Input::Char('D') => Some(Action::BeginSelectedWicDeviceWrite),
        Input::Char('x') => Some(Action::BeginActiveImageRuntimeCancellation),
        Input::Char('[') => Some(Action::SelectWicOutput { delta: -1 }),
        Input::Char(']') => Some(Action::SelectWicOutput { delta: 1 }),
        Input::Char('O') => Some(Action::OpenSelectedWicOutput),
        Input::Char('o') => Some(Action::OpenSelectedImageArtifact),
        Input::Char('m') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Manifest,
        )),
        Input::Char('l') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::License,
        )),
        Input::Char('s') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Spdx,
        )),
        Input::Char('w') => Some(Action::OpenSelectedImageArtifactAssociation(
            yoctui_model::ImageArtifactAssociation::Wic,
        )),
        _ => None,
    }
}

pub fn sdk_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkArtifactQuery(character)),
            Input::Backspace => Some(Action::BackspaceSdkArtifactQuery),
            Input::Enter | Input::Esc => Some(Action::FinishSdkArtifactSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkArtifact { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkArtifact { delta: 1 }),
        Input::Char('/') => Some(Action::BeginSdkArtifactSearch),
        Input::Char('R') => Some(Action::RefreshSdkArtifactInventory),
        Input::Char('s') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Standard,
        ))),
        Input::Char('E') => Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Extensible,
        ))),
        Input::Char('t') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Standard,
        ))),
        Input::Char('T') => Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Extensible,
        ))),
        Input::Char('P') => Some(Action::BeginSelectedSdkPublish),
        Input::Char('n') => Some(Action::BeginSdkNative),
        Input::Char('o') => Some(Action::OpenSelectedSdkArtifact),
        Input::Char('c') => Some(Action::BeginActiveSdkSessionCancellation),
        _ => None,
    }
}

pub fn sdk_build_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkBuild),
        Input::Esc => Some(Action::CancelSdkBuild),
        _ => None,
    }
}

pub fn sdk_publish_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendSdkPublishDestination(character)),
        Input::Backspace => Some(Action::BackspaceSdkPublishDestination),
        Input::Enter => Some(Action::PreviewSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublish),
        _ => None,
    }
}

pub fn sdk_publish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkPublish),
        Input::Esc => Some(Action::CancelSdkPublishPreview),
        _ => None,
    }
}

pub fn sdk_native_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendSdkNativeField(character)),
            Input::Backspace => Some(Action::BackspaceSdkNativeField),
            Input::Enter => Some(Action::FinishSdkNativeFieldEdit),
            Input::Esc => Some(Action::CancelSdkNative),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectSdkNativeField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectSdkNativeField { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') => {
            Some(Action::CycleSdkNativeMode)
        }
        Input::Enter => Some(Action::ActivateSdkNativeField),
        Input::Char('p') => Some(Action::PreviewSdkNative),
        Input::Esc => Some(Action::CancelSdkNative),
        _ => None,
    }
}

pub fn sdk_native_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkNative),
        Input::Esc => Some(Action::CancelSdkNativePreview),
        _ => None,
    }
}

pub fn sdk_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmSdkSessionCancellation),
        Input::Esc => Some(Action::CancelSdkSessionCancellation),
        _ => None,
    }
}

pub fn testing_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') => Some(Action::SelectTestFamily { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestFamily { delta: 1 }),
        Input::Enter | Input::Char('r') => Some(Action::BeginSelectedTestLaunch),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn security_workspace_action(
    view: SecurityView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    if searching {
        return match key {
            Input::Char(character) => security(SecurityAction::AppendQuery(character)),
            Input::Backspace => security(SecurityAction::BackspaceQuery),
            Input::Enter | Input::Esc => security(SecurityAction::FinishSearch),
            _ => None,
        };
    }
    match key {
        Input::Tab => security(SecurityAction::CycleView),
        Input::Up | Input::Char('k') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(-1)
        } else if drilled {
            SecurityAction::SelectComponent(-1)
        } else {
            SecurityAction::SelectReport(-1)
        }),
        Input::Down | Input::Char('j') => security(if view == SecurityView::Cves {
            SecurityAction::SelectFinding(1)
        } else if drilled {
            SecurityAction::SelectComponent(1)
        } else {
            SecurityAction::SelectReport(1)
        }),
        Input::Enter => security(SecurityAction::Drill),
        Input::Esc if drilled => security(SecurityAction::LeaveDrill),
        Input::Char('s') => security(SecurityAction::CycleScope),
        Input::Char('/') => security(SecurityAction::BeginSearch),
        Input::Char('f') => security(SecurityAction::CycleCveFilter),
        Input::Char('V') => security(SecurityAction::BeginCveCheck),
        Input::Char('M') => security(SecurityAction::BeginPackageMap),
        Input::Char('X') => security(SecurityAction::BeginSbomGeneration),
        Input::Char('I') => security(SecurityAction::BeginImport),
        Input::Char('R') => security(SecurityAction::RefreshReports),
        Input::Char('o') => security(SecurityAction::OpenSelectedReport),
        Input::Char('e') => security(SecurityAction::OpenSelectedRecipe),
        Input::Char('v') => security(SecurityAction::OpenSelectedAdvisory),
        Input::Char('c') => security(SecurityAction::BeginCancellation),
        _ => None,
    }
}

pub fn security_dialog_action(dialog: &SecurityDialog, key: Input) -> Option<Action> {
    let security = |action| Some(Action::Security(action));
    match dialog {
        SecurityDialog::Operation(preview) => match key {
            Input::Enter => security(SecurityAction::ConfirmOperation(preview.clone())),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Cancellation(id) => match key {
            Input::Enter => security(SecurityAction::ConfirmCancellation(*id)),
            Input::Esc => security(SecurityAction::CancelDialog),
            _ => None,
        },
        SecurityDialog::Import { editor, .. } => match key {
            Input::Enter => security(SecurityAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => {
                security(SecurityAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
    }
}

pub fn qa_workspace_action(
    view: QaView,
    drilled: bool,
    searching: bool,
    key: Input,
) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    if searching {
        return match key {
            Input::Char(character) => qa(QaAction::AppendQuery(character)),
            Input::Backspace => qa(QaAction::BackspaceQuery),
            Input::Enter | Input::Esc => qa(QaAction::FinishSearch),
            _ => None,
        };
    }
    match key {
        Input::Tab => qa(QaAction::CycleView),
        Input::Up | Input::Char('k') => qa(if drilled {
            QaAction::SelectFinding(-1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(-1)
        } else {
            QaAction::SelectCheck(-1)
        }),
        Input::Down | Input::Char('j') => qa(if drilled {
            QaAction::SelectFinding(1)
        } else if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::SelectCheck(1)
        }),
        Input::Enter => qa(QaAction::Drill),
        Input::Esc if drilled => qa(QaAction::LeaveDrill),
        Input::Char('s') => qa(if view == QaView::LayerQa {
            QaAction::SelectLayer(1)
        } else {
            QaAction::CycleScope
        }),
        Input::Char('/') => qa(QaAction::BeginSearch),
        Input::Char('f') => qa(QaAction::CycleStatusFilter),
        Input::Char('r') => qa(if view == QaView::LayerQa {
            QaAction::BeginSelectedLayerCheck
        } else {
            QaAction::BeginSelectedCheck
        }),
        Input::Char('I') => qa(QaAction::BeginImport),
        Input::Char('R') => qa(QaAction::RefreshReports),
        Input::Char('o') => qa(QaAction::OpenSelectedReport),
        Input::Char('e') => qa(if view == QaView::LayerQa {
            QaAction::OpenSelectedLayerRoot
        } else {
            QaAction::OpenProvider
        }),
        Input::Char('l') => qa(QaAction::OpenSelectedSource),
        Input::Char('c') => qa(if view == QaView::LayerQa {
            QaAction::BeginLayerCancellation
        } else {
            QaAction::BeginCancellation
        }),
        _ => None,
    }
}

pub fn qa_dialog_action(dialog: &QaDialog, key: Input) -> Option<Action> {
    let qa = |action| Some(Action::Qa(action));
    match dialog {
        QaDialog::Operation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerOperation(preview) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerOperation(preview.clone())),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Cancellation { session, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::LayerCancellation(session) => match key {
            Input::Enter => qa(QaAction::ConfirmLayerCancellation(*session)),
            Input::Esc => qa(QaAction::CancelDialog),
            _ => None,
        },
        QaDialog::Import { editor, .. } => match key {
            Input::Enter => qa(QaAction::ConfirmImport(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => qa(QaAction::CancelDialog),
            input => popup_editor_action(editor.editing, input),
        },
    }
}

pub fn maintenance_workspace_action(
    view: MaintenanceView,
    row_count: usize,
    key: Input,
) -> Option<Action> {
    let maintenance = |action| Some(Action::Maintenance(action));
    match key {
        Input::Char('[') => maintenance(MaintenanceAction::CycleView { backwards: true }),
        Input::Char(']') | Input::Tab => {
            maintenance(MaintenanceAction::CycleView { backwards: false })
        }
        Input::Up | Input::Char('k') => maintenance(MaintenanceAction::Select {
            delta: -1,
            row_count,
        }),
        Input::Down | Input::Char('j') => maintenance(MaintenanceAction::Select {
            delta: 1,
            row_count,
        }),
        Input::Char('r') => maintenance(MaintenanceAction::InspectCapability),
        Input::Char('x') => maintenance(MaintenanceAction::BeginCancellation),
        Input::Char('o') => maintenance(MaintenanceAction::OpenSelectedEvidence),
        Input::Char('S') => maintenance(MaintenanceAction::OpenSignatures),
        Input::Char('c') if view == MaintenanceView::Sstate => {
            maintenance(MaintenanceAction::OpenReadinessForm)
        }
        Input::Char('d') if view == MaintenanceView::Sstate => {
            maintenance(MaintenanceAction::OpenCleanupForm)
        }
        Input::Char('e') if view == MaintenanceView::Services => maintenance(
            MaintenanceAction::OpenPrServiceForm(yoctui_model::PrServiceOperation::Export),
        ),
        Input::Char('m') if view == MaintenanceView::Services => maintenance(
            MaintenanceAction::OpenPrServiceForm(yoctui_model::PrServiceOperation::Import),
        ),
        Input::Char('l') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenLockedCacheForm)
        }
        Input::Char('h') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenBuildHistoryForm)
        }
        Input::Char('a') if view == MaintenanceView::Release => {
            maintenance(MaintenanceAction::OpenGitArchiveForm)
        }
        _ => None,
    }
}

pub fn maintenance_dialog_action(dialog: &MaintenanceDialog, key: Input) -> Option<Action> {
    let maintenance = |action| Some(Action::Maintenance(action));
    match dialog {
        MaintenanceDialog::ReadinessToml { editor, .. } => match key {
            Input::Enter => {
                maintenance(MaintenanceAction::ConfirmReadinessToml(editor.text.clone()))
            }
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::CleanupToml { editor, .. } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmCleanupToml(editor.text.clone())),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::PrServiceToml {
            operation, editor, ..
        } => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmPrServiceToml {
                operation: *operation,
                document: editor.text.clone(),
            }),
            Input::Char('q') | Input::Esc if !editor.editing => {
                maintenance(MaintenanceAction::CancelDialog)
            }
            input => popup_editor_action(editor.editing, input),
        },
        MaintenanceDialog::ReadinessForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Left | Input::Right | Input::Char(' ')
                    if next.field == MaintenanceReadinessField::Mode =>
                {
                    next.mode = match next.mode {
                        yoctui_model::SstateReadinessMode::IsolatedTmpdir => {
                            yoctui_model::SstateReadinessMode::SameTmpdir
                        }
                        yoctui_model::SstateReadinessMode::SameTmpdir => {
                            yoctui_model::SstateReadinessMode::IsolatedTmpdir
                        }
                    };
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceReadinessField::Targets => &mut next.targets,
                        MaintenanceReadinessField::Output => &mut next.output,
                        MaintenanceReadinessField::Log => &mut next.log,
                        MaintenanceReadinessField::Timeout if character.is_ascii_digit() => {
                            &mut next.timeout
                        }
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => match next.field {
                    MaintenanceReadinessField::Targets => {
                        next.targets.pop();
                    }
                    MaintenanceReadinessField::Output => {
                        next.output.pop();
                    }
                    MaintenanceReadinessField::Log => {
                        next.log.pop();
                    }
                    MaintenanceReadinessField::Timeout => {
                        next.timeout.pop();
                    }
                    MaintenanceReadinessField::Mode => return None,
                },
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmReadinessForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateReadinessForm(Box::new(next)))
        }
        MaintenanceDialog::CleanupForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right => match next.field {
                    MaintenanceCleanupField::Duplicates => next.duplicates = !next.duplicates,
                    MaintenanceCleanupField::Orphans => next.orphans = !next.orphans,
                    MaintenanceCleanupField::UnreferencedByStamps => {
                        next.unreferenced_by_stamps = !next.unreferenced_by_stamps;
                    }
                    MaintenanceCleanupField::Jobs => return None,
                },
                Input::Char(character)
                    if next.field == MaintenanceCleanupField::Jobs
                        && character.is_ascii_digit()
                        && next.jobs.len() < 5 =>
                {
                    next.jobs.push(character);
                }
                Input::Backspace if next.field == MaintenanceCleanupField::Jobs => {
                    next.jobs.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmCleanupForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateCleanupForm(Box::new(next)))
        }
        MaintenanceDialog::PrServiceForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Char(character) if !character.is_control() => {
                    if next.file.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        next.file.push(character);
                    }
                }
                Input::Backspace => {
                    next.file.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmPrServiceForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdatePrServiceForm(Box::new(next)))
        }
        MaintenanceDialog::LockedCacheForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceLockedCacheField::LockedSignatures => {
                            &mut next.locked_signatures
                        }
                        MaintenanceLockedCacheField::InputCache => &mut next.input_cache,
                        MaintenanceLockedCacheField::OutputCache => &mut next.output_cache,
                        MaintenanceLockedCacheField::Filter => &mut next.filter,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    match next.field {
                        MaintenanceLockedCacheField::LockedSignatures => {
                            next.locked_signatures.pop();
                        }
                        MaintenanceLockedCacheField::InputCache => {
                            next.input_cache.pop();
                        }
                        MaintenanceLockedCacheField::OutputCache => {
                            next.output_cache.pop();
                        }
                        MaintenanceLockedCacheField::Filter => {
                            next.filter.pop();
                        }
                    };
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmLockedCacheForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateLockedCacheForm(Box::new(next)))
        }
        MaintenanceDialog::BuildHistoryForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right if next.field.is_toggle() => {
                    match next.field {
                        MaintenanceBuildHistoryField::ReportVersion => {
                            next.report_version = !next.report_version;
                        }
                        MaintenanceBuildHistoryField::ReportAll => {
                            next.report_all = !next.report_all;
                        }
                        MaintenanceBuildHistoryField::Signatures => {
                            next.signatures = !next.signatures;
                        }
                        MaintenanceBuildHistoryField::SignatureDiff => {
                            next.signature_diff = !next.signature_diff;
                        }
                        MaintenanceBuildHistoryField::NoColour => {
                            next.no_colour = !next.no_colour;
                        }
                        _ => return None,
                    }
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceBuildHistoryField::FromRevision => &mut next.from_revision,
                        MaintenanceBuildHistoryField::ToRevision => &mut next.to_revision,
                        MaintenanceBuildHistoryField::ExcludePaths => &mut next.exclude_paths,
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    match next.field {
                        MaintenanceBuildHistoryField::FromRevision => {
                            next.from_revision.pop();
                        }
                        MaintenanceBuildHistoryField::ToRevision => {
                            next.to_revision.pop();
                        }
                        MaintenanceBuildHistoryField::ExcludePaths => {
                            next.exclude_paths.pop();
                        }
                        _ => return None,
                    };
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmBuildHistoryForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateBuildHistoryForm(Box::new(next)))
        }
        MaintenanceDialog::GitArchiveForm(draft) => {
            let mut next = (**draft).clone();
            next.validation = None;
            match key {
                Input::Tab => next.field = next.field.cycle(false),
                Input::BackTab => next.field = next.field.cycle(true),
                Input::Char(' ') | Input::Left | Input::Right if next.field.is_toggle() => {
                    match next.field {
                        MaintenanceGitArchiveField::Create => next.create = !next.create,
                        MaintenanceGitArchiveField::Bare => next.bare = !next.bare,
                        MaintenanceGitArchiveField::CreateTag => {
                            next.create_tag = !next.create_tag;
                        }
                        _ => return None,
                    }
                }
                Input::Char(character) if !character.is_control() => {
                    let value = match next.field {
                        MaintenanceGitArchiveField::DataDir => &mut next.data_dir,
                        MaintenanceGitArchiveField::GitDir => &mut next.git_dir,
                        MaintenanceGitArchiveField::BranchName => &mut next.branch_name,
                        MaintenanceGitArchiveField::TagName => &mut next.tag_name,
                        MaintenanceGitArchiveField::CommitSubject => &mut next.commit_subject,
                        MaintenanceGitArchiveField::CommitBody => &mut next.commit_body,
                        MaintenanceGitArchiveField::TagSubject => &mut next.tag_subject,
                        MaintenanceGitArchiveField::TagBody => &mut next.tag_body,
                        MaintenanceGitArchiveField::Exclusions => &mut next.exclusions,
                        MaintenanceGitArchiveField::Notes => &mut next.notes,
                        MaintenanceGitArchiveField::PushRemote => &mut next.push_remote,
                        _ => return None,
                    };
                    if value.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES
                    {
                        value.push(character);
                    }
                }
                Input::Backspace => {
                    let value = match next.field {
                        MaintenanceGitArchiveField::DataDir => &mut next.data_dir,
                        MaintenanceGitArchiveField::GitDir => &mut next.git_dir,
                        MaintenanceGitArchiveField::BranchName => &mut next.branch_name,
                        MaintenanceGitArchiveField::TagName => &mut next.tag_name,
                        MaintenanceGitArchiveField::CommitSubject => &mut next.commit_subject,
                        MaintenanceGitArchiveField::CommitBody => &mut next.commit_body,
                        MaintenanceGitArchiveField::TagSubject => &mut next.tag_subject,
                        MaintenanceGitArchiveField::TagBody => &mut next.tag_body,
                        MaintenanceGitArchiveField::Exclusions => &mut next.exclusions,
                        MaintenanceGitArchiveField::Notes => &mut next.notes,
                        MaintenanceGitArchiveField::PushRemote => &mut next.push_remote,
                        _ => return None,
                    };
                    value.pop();
                }
                Input::Enter => {
                    return maintenance(MaintenanceAction::ConfirmGitArchiveForm(Box::new(next)));
                }
                Input::Esc => return maintenance(MaintenanceAction::CancelDialog),
                _ => return None,
            }
            maintenance(MaintenanceAction::UpdateGitArchiveForm(Box::new(next)))
        }
        MaintenanceDialog::Confirm(preview) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmOperation(preview.clone())),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::CleanupPhrase { preview, input } => match key {
            Input::Char(character)
                if !character.is_control()
                    && input.len() + character.len_utf8()
                        <= yoctui_model::MAX_MAINTENANCE_TEXT_BYTES =>
            {
                let mut next = input.clone();
                next.push(character);
                maintenance(MaintenanceAction::UpdateCleanupPhrase {
                    preview: preview.clone(),
                    input: next,
                })
            }
            Input::Backspace => {
                let mut next = input.clone();
                next.pop();
                maintenance(MaintenanceAction::UpdateCleanupPhrase {
                    preview: preview.clone(),
                    input: next,
                })
            }
            Input::Enter => maintenance(MaintenanceAction::ConfirmCleanupPhrase {
                preview: preview.clone(),
                input: input.clone(),
            }),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::ConfirmNetworkPush(preview) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmNetworkPush(preview.clone())),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
        MaintenanceDialog::ConfirmCancellation(id) => match key {
            Input::Enter => maintenance(MaintenanceAction::ConfirmCancellation(*id)),
            Input::Esc => maintenance(MaintenanceAction::CancelDialog),
            _ => None,
        },
    }
}

pub fn test_launch_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendTestLaunchField(character)),
            Input::Backspace => Some(Action::BackspaceTestLaunchField),
            Input::Enter => Some(Action::FinishTestLaunchFieldEdit),
            Input::Esc => Some(Action::CancelTestLaunch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectTestLaunchField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestLaunchField { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') | Input::Enter => {
            Some(Action::ActivateTestLaunchField)
        }
        Input::Char('p') => Some(Action::PreviewTestLaunch),
        Input::Esc => Some(Action::CancelTestLaunch),
        _ => None,
    }
}

pub fn test_launch_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestLaunch),
        Input::Esc => Some(Action::CancelTestLaunchPreview),
        _ => None,
    }
}

pub fn test_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestSessionCancellation),
        Input::Esc => Some(Action::CancelTestSessionCancellation),
        _ => None,
    }
}

pub fn test_results_workspace_action(searching: bool, drilled: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendTestResultQuery(character)),
            Input::Backspace => Some(Action::BackspaceTestResultQuery),
            Input::Enter | Input::Esc => Some(Action::FinishTestResultSearch),
            _ => None,
        };
    }
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') if drilled => Some(Action::SelectTestCase { delta: -1 }),
        Input::Down | Input::Char('j') if drilled => Some(Action::SelectTestCase { delta: 1 }),
        Input::Up | Input::Char('k') => Some(Action::SelectTestResult { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestResult { delta: 1 }),
        Input::Enter if drilled => None,
        Input::Enter => Some(Action::DrillIntoSelectedTestResult),
        Input::Esc if drilled => Some(Action::LeaveTestResultCases),
        Input::Char('/') => Some(Action::BeginTestResultSearch),
        Input::Char('I') => Some(Action::BeginTestResultImport),
        Input::Char('R') => Some(Action::RefreshTestResults),
        Input::Char('c') => Some(Action::BeginTestComparison),
        Input::Char('J') => Some(Action::BeginTestJunitExport),
        Input::Char('o') => Some(Action::OpenSelectedTestResult),
        Input::Char('l') => Some(Action::OpenSelectedTestCaseLog),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn test_comparison_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Tab => Some(Action::CycleTestView),
        Input::Up | Input::Char('k') => Some(Action::SelectTestComparisonTransition { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestComparisonTransition { delta: 1 }),
        Input::Char('c') => Some(Action::BeginTestComparison),
        Input::Char('l') => Some(Action::OpenSelectedTestTransitionLog),
        Input::Char('x') => Some(Action::BeginActiveTestSessionCancellation),
        _ => None,
    }
}

pub fn test_result_import_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendTestResultImport(character)),
        Input::Backspace => Some(Action::BackspaceTestResultImport),
        Input::Enter => Some(Action::ConfirmTestResultImport),
        Input::Esc => Some(Action::CancelTestResultImport),
        _ => None,
    }
}

pub fn test_comparison_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectTestComparisonChoice { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectTestComparisonChoice { delta: 1 }),
        Input::Left | Input::Right | Input::Char('h') | Input::Char('l') => {
            Some(Action::CycleTestComparisonField)
        }
        Input::Enter => Some(Action::ActivateTestComparisonChoice),
        Input::Char('p') => Some(Action::PreviewTestComparison),
        Input::Esc => Some(Action::CancelTestComparison),
        _ => None,
    }
}

pub fn test_comparison_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestComparison),
        Input::Esc => Some(Action::CancelTestComparisonPreview),
        _ => None,
    }
}

pub fn test_junit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendTestJunitDestination(character)),
        Input::Backspace => Some(Action::BackspaceTestJunitDestination),
        Input::Enter => Some(Action::PreviewTestJunitExport),
        Input::Esc => Some(Action::CancelTestJunitExport),
        _ => None,
    }
}

pub fn test_junit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmTestJunitExport),
        Input::Esc => Some(Action::CancelTestJunitExportPreview),
        _ => None,
    }
}

pub fn wic_create_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendWicCreateField(character)),
            Input::Backspace => Some(Action::BackspaceWicCreateField),
            Input::Enter => Some(Action::FinishWicCreateFieldEdit),
            Input::Esc => Some(Action::CancelWicCreate),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectWicCreateField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectWicCreateField { delta: 1 }),
        Input::Left | Input::Char('h') => Some(Action::CycleWicCreateChoice { backwards: true }),
        Input::Right | Input::Char('l') => Some(Action::CycleWicCreateChoice { backwards: false }),
        Input::Enter => Some(Action::ActivateWicCreateField),
        Input::Char('p') => Some(Action::PreviewWicCreate),
        Input::Esc => Some(Action::CancelWicCreate),
        _ => None,
    }
}

pub fn wic_create_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicCreate),
        Input::Esc => Some(Action::CancelWicCreatePreview),
        _ => None,
    }
}

pub fn wic_device_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectWicDevice { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectWicDevice { delta: 1 }),
        Input::Enter => Some(Action::ConfirmWicDeviceSelection),
        Input::Esc => Some(Action::CancelWicDevicePicker),
        _ => None,
    }
}

pub fn wic_write_phrase_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendWicWritePhrase(character)),
        Input::Backspace => Some(Action::BackspaceWicWritePhrase),
        Input::Enter => Some(Action::PreviewWicDeviceWrite),
        Input::Esc => Some(Action::CancelWicWritePhrase),
        _ => None,
    }
}

pub fn wic_write_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicDeviceWrite),
        Input::Esc => Some(Action::CancelWicWritePreview),
        _ => None,
    }
}

pub fn wic_cancellation_confirmation_action(
    id: WicSessionId,
    incomplete_device_warning: bool,
    key: Input,
) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmWicSessionCancellation {
            id,
            acknowledge_incomplete_device: incomplete_device_warning,
        }),
        Input::Esc => Some(Action::CancelWicSessionCancellation),
        _ => None,
    }
}

pub fn qemu_launch_dialog_action(editing: bool, key: Input) -> Option<Action> {
    if editing {
        return match key {
            Input::Char(character) => Some(Action::AppendQemuLaunchField(character)),
            Input::Backspace => Some(Action::BackspaceQemuLaunchField),
            Input::Enter => Some(Action::FinishQemuLaunchFieldEdit),
            Input::Esc => Some(Action::CancelQemuLaunch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectQemuLaunchField { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectQemuLaunchField { delta: 1 }),
        Input::Left | Input::Char('h') => Some(Action::CycleQemuLaunchChoice { backwards: true }),
        Input::Right | Input::Char('l') => Some(Action::CycleQemuLaunchChoice { backwards: false }),
        Input::Enter => Some(Action::ActivateQemuLaunchField),
        Input::Char('p') => Some(Action::PreviewQemuLaunch),
        Input::Esc => Some(Action::CancelQemuLaunch),
        _ => None,
    }
}

pub fn qemu_launch_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmQemuLaunch),
        Input::Esc => Some(Action::CancelQemuLaunchPreview),
        _ => None,
    }
}

pub fn qemu_cancellation_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmQemuSessionCancellation),
        Input::Esc => Some(Action::CancelQemuSessionCancellation),
        _ => None,
    }
}

pub fn layer_tree_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectLayerBrowserEntry { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectLayerBrowserEntry { delta: 1 }),
        Input::Enter => Some(Action::LayerBrowserEnter),
        Input::Right | Input::Char('l') => Some(Action::LayerBrowserExpand),
        Input::Left | Input::Char('h') => Some(Action::LayerBrowserUp),
        Input::Esc => Some(Action::CloseLayerBrowser),
        Input::Char('r') => Some(Action::RefreshLayerBrowser),
        Input::Char('e') => Some(Action::EditSelectedLayerBrowserFile),
        Input::Char('.') => Some(Action::ToggleLayerBrowserHidden),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('g') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Git)),
        Input::Char('m') => Some(Action::SetLayerInspectorMode(LayerInspectorMode::Metadata)),
        Input::Char('d') => Some(Action::SetLayerInspectorMode(
            LayerInspectorMode::Dependencies,
        )),
        _ => None,
    }
}
pub fn recipes_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectRecipe { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectRecipe { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedRecipeMetadata),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('e') => Some(Action::OpenSelectedRecipeProvider),
        Input::Char('o') => Some(Action::BeginSelectedRecipeTaskLog),
        Input::Char('p') => Some(Action::BeginSelectedRecipePatchReview),
        Input::Char('g') => Some(Action::BeginSelectedRecipeDependencies),
        Input::Char('f') => Some(Action::BeginSelectedRecipeForceTask),
        Input::Char('v') => Some(Action::BeginSelectedRecipeDevshell),
        Input::Char('K') => Some(Action::BeginSelectedRecipeDiffconfig),
        Input::Char('z') => Some(Action::BeginSelectedRecipeDiffsigs),
        Input::Char('Z') => Some(Action::BeginSelectedRecipeSignatures),
        Input::Char('V') => Some(Action::BeginSelectedRecipeCveCheck),
        Input::Char('X') => Some(Action::BeginSelectedRecipeSpdx),
        Input::Char('d') => Some(Action::BeginSelectedRecipeDevtoolModify),
        Input::Char('t') => Some(Action::BeginSelectedRecipeDevtoolStatus),
        Input::Char('u') => Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        Input::Char('F') => Some(Action::BeginSelectedRecipeDevtoolFinish),
        Input::Char('P') => Some(Action::BeginSelectedRecipeDevtoolDeploy),
        Input::Char('D') => Some(Action::BeginSelectedRecipeDevtoolReset),
        _ => None,
    }
}

pub fn devtool_modify_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolModify),
        Input::Esc => Some(Action::CancelDevtoolModify),
        _ => None,
    }
}

pub fn devtool_update_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolUpdateRecipe),
        Input::Esc => Some(Action::CancelDevtoolUpdateRecipe),
        _ => None,
    }
}

pub fn devtool_finish_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectDevtoolFinishLayer { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectDevtoolFinishLayer { delta: 1 }),
        Input::Enter => Some(Action::PreviewDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinish),
        _ => None,
    }
}

pub fn devtool_finish_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolFinish),
        Input::Esc => Some(Action::CancelDevtoolFinishConfirmation),
        _ => None,
    }
}

pub fn devtool_deploy_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendDevtoolDeployTarget(character)),
        Input::Backspace => Some(Action::BackspaceDevtoolDeployTarget),
        Input::Enter => Some(Action::PreviewDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeploy),
        _ => None,
    }
}

pub fn devtool_deploy_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolDeploy),
        Input::Esc => Some(Action::CancelDevtoolDeployConfirmation),
        _ => None,
    }
}

pub fn devtool_reset_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmDevtoolReset),
        Input::Esc => Some(Action::CancelDevtoolReset),
        _ => None,
    }
}

pub fn recipe_editor_action(editing: bool, key: Input) -> Option<Action> {
    match key {
        Input::Esc => Some(Action::CloseRecipeEditor),
        Input::Up => Some(Action::SelectRecipeEditorFile { delta: -1 }),
        Input::Down => Some(Action::SelectRecipeEditorFile { delta: 1 }),
        Input::Enter if editing => Some(Action::AppendRecipeEditor('\n')),
        Input::Enter | Input::Char('e') if !editing => Some(Action::ToggleRecipeEditorEditing),
        Input::CtrlS => Some(Action::SaveRecipeEditor),
        Input::CtrlB => Some(Action::BeginRecipeEditorBuild),
        Input::Backspace => Some(Action::BackspaceRecipeEditor),
        Input::Char(character) => Some(Action::AppendRecipeEditor(character)),
        _ => None,
    }
}

pub fn config_workspace_action(searching: bool, key: Input) -> Option<Action> {
    if searching {
        return match key {
            Input::Char(character) => Some(Action::AppendMetadataQuery(character)),
            Input::Backspace => Some(Action::BackspaceMetadataQuery),
            Input::Enter | Input::Esc => Some(Action::FinishMetadataSearch),
            _ => None,
        };
    }
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigVariable { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigVariable { delta: 1 }),
        Input::Enter => Some(Action::BeginSelectedConfigDetail),
        Input::Char('C') => Some(Action::CopySelectedConfigEffective),
        Input::Char('U') => Some(Action::CopySelectedConfigUnexpanded),
        Input::Char('s') => Some(Action::OpenConfigScopePicker),
        Input::Char('c') => Some(Action::OpenConfigComparison),
        Input::Char('E') => Some(Action::BeginConfigEdit),
        Input::Char('/') => Some(Action::BeginMetadataSearch),
        Input::Char('o') => Some(Action::OpenSelectedConfigSource),
        _ => None,
    }
}

pub fn config_source_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigSource { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigSource { delta: 1 }),
        Input::Enter => Some(Action::OpenSelectedConfigSourceChoice),
        Input::Esc => Some(Action::CancelConfigSourcePicker),
        _ => None,
    }
}

pub fn config_scope_picker_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectConfigScope { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectConfigScope { delta: 1 }),
        Input::Enter => Some(Action::ConfirmConfigScope),
        Input::Esc => Some(Action::CancelConfigScopePicker),
        _ => None,
    }
}

pub fn config_compare_dialog_action(key: Input) -> Option<Action> {
    matches!(key, Input::Enter | Input::Esc).then_some(Action::CloseConfigComparison)
}

pub fn config_edit_dialog_action(key: Input) -> Option<Action> {
    match key {
        Input::Char(character) => Some(Action::AppendConfigEdit(character)),
        Input::Backspace => Some(Action::BackspaceConfigEdit),
        Input::Enter => Some(Action::PreviewConfigEdit),
        Input::Esc => Some(Action::CancelConfigEdit),
        _ => None,
    }
}

pub fn config_edit_confirmation_action(key: Input) -> Option<Action> {
    match key {
        Input::Enter => Some(Action::ConfirmConfigEdit),
        Input::Esc => Some(Action::CancelConfigEditConfirmation),
        _ => None,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::PathBuf, time::Duration};
    use yoctui_model::{
        App, BackgroundJobStatus, BuildStatus, DependencyEdge, DependencyEdgeKind, DependencyGraph,
        DependencyGraphState, DependencyNodeId, ImageArtifact, ImageArtifactField,
        ImageArtifactIdentity, ImageArtifactInventory, ImageArtifactKind, ImageArtifactRequest,
        PackageDetail, PackageDetailRequest, PackageField, PackageIdentity,
        PackageInventoryRequest, PackageSummary, SignatureComparisonRequest, SignatureDifference,
        SignatureDifferenceCategory, SignatureIdentity, SignatureRecord, SignatureTarget, update,
    };

    fn apply_actions(app: &mut App, actions: Vec<Action>) {
        for action in actions {
            let _ = update(app, action);
        }
    }

    fn request() -> BuildRequest {
        BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }
    }

    fn maintenance_preview() -> yoctui_model::MaintenanceOperationPreview {
        yoctui_model::MaintenanceOperationPreview::new(
            7,
            1,
            yoctui_model::MaintenanceOperation::SstateReadiness(
                yoctui_model::SstateReadinessRequest::new(
                    vec!["core-image-minimal".into()],
                    yoctui_model::SstateReadinessMode::IsolatedTmpdir,
                    None,
                    None,
                    60,
                )
                .unwrap(),
            ),
            vec![
                "0: /tools/oe-check-sstate".into(),
                "1: core-image-minimal".into(),
            ],
            vec![],
        )
        .unwrap()
    }

    #[test]
    fn maintenance_workflow_maps_first_class_screen_and_typed_keys() {
        let mut app = App::new(10, 1_000);
        assert_eq!(
            update(&mut app, Action::Open(Screen::Maintenance)),
            Some(yoctui_model::Effect::Maintenance(
                yoctui_model::MaintenanceEffect::InspectCapability { request: 1 }
            ))
        );
        assert_eq!(app.screen, Screen::Maintenance);
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char(']')),
            Some(Action::Maintenance(MaintenanceAction::CycleView {
                backwards: false
            }))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Down),
            Some(Action::Maintenance(MaintenanceAction::Select {
                delta: 1,
                row_count: 4
            }))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char('S')),
            Some(Action::Maintenance(MaintenanceAction::OpenSignatures))
        );
    }

    #[test]
    fn maintenance_workflow_dialog_mapping_traps_typed_input() {
        let preview = maintenance_preview();
        assert_eq!(
            maintenance_dialog_action(&MaintenanceDialog::Confirm(preview.clone()), Input::Enter),
            Some(Action::Maintenance(MaintenanceAction::ConfirmOperation(
                preview.clone()
            )))
        );
        assert_eq!(
            maintenance_dialog_action(
                &MaintenanceDialog::CleanupPhrase {
                    preview: preview.clone(),
                    input: "DELETE".into()
                },
                Input::Char(' ')
            ),
            Some(Action::Maintenance(
                MaintenanceAction::UpdateCleanupPhrase {
                    preview: preview.clone(),
                    input: "DELETE ".into()
                }
            ))
        );
        assert_eq!(
            maintenance_dialog_action(&MaintenanceDialog::ConfirmNetworkPush(preview), Input::Esc),
            Some(Action::Maintenance(MaintenanceAction::CancelDialog))
        );
    }

    #[test]
    fn maintenance_sstate_workspace_maps_only_typed_form_input() {
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('c')),
            Some(Action::Maintenance(MaintenanceAction::OpenReadinessForm))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('d')),
            Some(Action::Maintenance(MaintenanceAction::OpenCleanupForm))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Release, 4, Input::Char('c')),
            None
        );

        let mut readiness = yoctui_model::PopupEditor::new(
            "targets = \"\"\nmode = \"isolated_tmpdir\"\ntimeout = 3600\n".into(),
        );
        readiness.select_range(11, 11);
        readiness.editing = true;
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::ReadinessToml {
                    editor: readiness.clone(),
                    validation_error: None,
                },
                Input::Char('c'),
            ),
            Some(Action::EditActivePopup(PopupEditorCommand::Insert('c')))
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::ReadinessToml {
                    editor: readiness.clone(),
                    validation_error: None,
                },
                Input::Enter,
            ),
            Some(Action::Maintenance(
                MaintenanceAction::ConfirmReadinessToml(document)
            )) if document == readiness.text
        ));

        let cleanup = yoctui_model::PopupEditor::new(
            "duplicates = true\norphans = false\nunreferenced_by_stamps = false\njobs = 1\n".into(),
        );
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::CleanupToml {
                    editor: cleanup.clone(),
                    validation_error: None,
                },
                Input::Char('e'),
            ),
            Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::CleanupToml {
                    editor: cleanup.clone(),
                    validation_error: None,
                },
                Input::Enter,
            ),
            Some(Action::Maintenance(MaintenanceAction::ConfirmCleanupToml(document)))
                if document == cleanup.text
        ));
        assert_eq!(
            maintenance_dialog_action(
                &MaintenanceDialog::CleanupToml {
                    editor: cleanup,
                    validation_error: None,
                },
                Input::Esc,
            ),
            Some(Action::Maintenance(MaintenanceAction::CancelDialog))
        );
    }

    #[test]
    fn maintenance_service_workspace_maps_distinct_export_and_import_forms() {
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('e')),
            Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
                yoctui_model::PrServiceOperation::Export,
            )))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('m')),
            Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
                yoctui_model::PrServiceOperation::Import,
            )))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('e')),
            None
        );
        let mut editor = yoctui_model::PopupEditor::new("file = \"\"\n".into());
        editor.select_range(8, 8);
        editor.editing = true;
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::PrServiceToml {
                    operation: yoctui_model::PrServiceOperation::Import,
                    editor: editor.clone(),
                    validation_error: None,
                },
                Input::Char('/'),
            ),
            Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::PrServiceToml {
                    operation: yoctui_model::PrServiceOperation::Import,
                    editor: editor.clone(),
                    validation_error: None,
                },
                Input::Enter,
            ),
            Some(Action::Maintenance(MaintenanceAction::ConfirmPrServiceToml {
                operation: yoctui_model::PrServiceOperation::Import,
                document,
            })) if document == editor.text
        ));
    }

    #[test]
    fn maintenance_release_locked_workspace_maps_only_typed_form_input() {
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('l')),
            Some(Action::Maintenance(MaintenanceAction::OpenLockedCacheForm))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('l')),
            None
        );
        let metadata = yoctui_model::MaintenanceMetadata {
            native_lsb: Some("ubuntu".into()),
            ..yoctui_model::MaintenanceMetadata::default()
        };
        let draft = yoctui_model::MaintenanceLockedCacheDraft::from_metadata(&metadata).unwrap();
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::LockedCacheForm(Box::new(draft.clone())),
                Input::Char('/'),
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateLockedCacheForm(next)))
                if next.locked_signatures == "/"
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::LockedCacheForm(Box::new(draft.clone())),
                Input::Tab,
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateLockedCacheForm(next)))
                if next.field == MaintenanceLockedCacheField::InputCache
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::LockedCacheForm(Box::new(draft.clone())),
                Input::Enter,
            ),
            Some(Action::Maintenance(
                MaintenanceAction::ConfirmLockedCacheForm(_)
            ))
        ));
        assert_eq!(
            maintenance_dialog_action(
                &MaintenanceDialog::LockedCacheForm(Box::new(draft)),
                Input::Esc,
            ),
            Some(Action::Maintenance(MaintenanceAction::CancelDialog))
        );
    }

    #[test]
    fn maintenance_release_history_workspace_maps_fields_and_toggles_without_leakage() {
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('h')),
            Some(Action::Maintenance(MaintenanceAction::OpenBuildHistoryForm))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Sstate, 3, Input::Char('h')),
            None
        );
        let metadata = yoctui_model::MaintenanceMetadata {
            buildhistory_dir: Some("/build/buildhistory".into()),
            ..yoctui_model::MaintenanceMetadata::default()
        };
        let draft = yoctui_model::MaintenanceBuildHistoryDraft::from_metadata(&metadata).unwrap();
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::BuildHistoryForm(Box::new(draft.clone())),
                Input::Char('H'),
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateBuildHistoryForm(next)))
                if next.from_revision == "H"
        ));
        let mut toggle = draft.clone();
        toggle.field = MaintenanceBuildHistoryField::ReportVersion;
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::BuildHistoryForm(Box::new(toggle)),
                Input::Char(' '),
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateBuildHistoryForm(next)))
                if next.report_version
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::BuildHistoryForm(Box::new(draft.clone())),
                Input::Enter,
            ),
            Some(Action::Maintenance(
                MaintenanceAction::ConfirmBuildHistoryForm(_)
            ))
        ));
        assert_eq!(
            maintenance_dialog_action(
                &MaintenanceDialog::BuildHistoryForm(Box::new(draft)),
                Input::Esc,
            ),
            Some(Action::Maintenance(MaintenanceAction::CancelDialog))
        );
    }

    #[test]
    fn maintenance_release_archive_workspace_maps_text_toggles_and_cancel() {
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('a')),
            Some(Action::Maintenance(MaintenanceAction::OpenGitArchiveForm))
        );
        assert_eq!(
            maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('a')),
            None
        );
        let draft = yoctui_model::MaintenanceGitArchiveDraft::default();
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::GitArchiveForm(Box::new(draft.clone())),
                Input::Char('/'),
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateGitArchiveForm(next)))
                if next.data_dir == "/"
        ));
        let mut toggle = draft.clone();
        toggle.field = MaintenanceGitArchiveField::Bare;
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::GitArchiveForm(Box::new(toggle)),
                Input::Right,
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateGitArchiveForm(next)))
                if next.bare
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::GitArchiveForm(Box::new(draft.clone())),
                Input::BackTab,
            ),
            Some(Action::Maintenance(MaintenanceAction::UpdateGitArchiveForm(next)))
                if next.field == MaintenanceGitArchiveField::PushRemote
        ));
        assert!(matches!(
            maintenance_dialog_action(
                &MaintenanceDialog::GitArchiveForm(Box::new(draft.clone())),
                Input::Enter,
            ),
            Some(Action::Maintenance(
                MaintenanceAction::ConfirmGitArchiveForm(_)
            ))
        ));
        assert_eq!(
            maintenance_dialog_action(
                &MaintenanceDialog::GitArchiveForm(Box::new(draft)),
                Input::Esc,
            ),
            Some(Action::Maintenance(MaintenanceAction::CancelDialog))
        );
    }

    #[test]
    fn background_job_build_events_survive_navigation_and_complete() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Starting
        );
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildStarted,
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let _ = update(&mut app, Action::Open(Screen::Layers));
        let log = yoctui_model::LogEntry {
            id: 0,
            severity: Severity::Warning,
            message: "cache miss".into(),
            recipe: Some("busybox".into()),
            task: Some("do_compile".into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            build: None,
            protected: false,
            diagnostic: None,
        };
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::Log(log),
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildCompleted {
                    success: true,
                    exit_code: Some(0),
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(3),
            ),
        );

        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(app.screen, Screen::Layers);
        assert_eq!(app.build.status, BuildStatus::Completed);
        assert_eq!(job.status, BackgroundJobStatus::Succeeded);
        assert_eq!(job.output.len(), 1);
        assert_eq!(job.warnings, 1);
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn typed_event_maps_every_metadata_family_and_ignores_future_events() {
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Recipes(vec![])),
            Some(Action::RecipesLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Layers(vec![])),
            Some(Action::LayersLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Variable {
                name: "MACHINE".into(),
                recipe: None,
                value: Some("qemux86-64".into()),
                provenance: Some("conf/local.conf:1".into()),
                unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
                operations: vec![],
                active_overrides: vec![],
            }),
            Some(Action::VariableLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::Dependencies {
                recipe: "busybox".into(),
                build: vec![],
                runtime: vec![],
            }),
            Some(Action::DependenciesLoaded(_))
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::RecipeSources {
                recipe: "busybox".into(),
                paths: vec!["/workspace/busybox".into()],
            }),
            Some(Action::RecipeSourcesLoaded { .. })
        ));
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::LayerRelationships(vec![])),
            Some(Action::LayerRelationshipsLoaded(_))
        ));
        assert_eq!(model_action_from_backend_event(BackendEvent::Ignored), None);
    }
    #[test]
    fn dependency_graph_typed_events_map_success_partial_and_failure() {
        let root = DependencyNodeId::recipe("core-image-minimal");
        let (graph, _) = DependencyGraph::normalize(
            root.clone(),
            Vec::new(),
            vec![DependencyEdge {
                from: root.clone(),
                to: DependencyNodeId::recipe("busybox"),
                kind: DependencyEdgeKind::Runtime,
            }],
            10,
            10,
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraph {
                graph: graph.clone(),
                limitations: Vec::new(),
            }),
            Some(Action::DependencyGraphLoaded(graph.clone()))
        );
        let mut compatibility = App::new(10, 1_000);
        let action = model_action_from_backend_event(BackendEvent::DependencyGraph {
            graph: graph.clone(),
            limitations: Vec::new(),
        })
        .unwrap();
        let _ = update(&mut compatibility, action);
        assert_eq!(
            compatibility.dependencies.as_ref().unwrap().runtime,
            ["busybox"]
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraph {
                graph: graph.clone(),
                limitations: vec!["task graph unavailable".into()],
            }),
            Some(Action::DependencyGraphPartial {
                graph,
                limitations: vec!["task graph unavailable".into()],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
                root: root.clone(),
                message: "query failed".into(),
            }),
            Some(Action::DependencyGraphFailed {
                root,
                message: "query failed".into(),
            })
        );

        let mut app = App::new(10, 1_000);
        let action = model_action_from_backend_event(BackendEvent::DependencyGraphFailed {
            root: DependencyNodeId::recipe("image"),
            message: "offline".into(),
        })
        .unwrap();
        let _ = update(&mut app, action);
        assert!(matches!(
            app.dependency_graph,
            DependencyGraphState::Failed { .. }
        ));
    }
    #[test]
    fn signature_model_typed_events_map_dump_comparison_partial_and_failure() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let identity = SignatureIdentity {
            target: target.clone(),
            hash: Some("abc".into()),
            path: Some("/tmp/busybox.sigdata".into()),
        };
        let record = SignatureRecord {
            identity: identity.clone(),
            base_hash: Some("base".into()),
            task_hash: Some("task".into()),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureDump {
                target: target.clone(),
                records: vec![record.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::SignatureDumpLoaded {
                target: target.clone(),
                records: vec![record.clone()],
            })
        );
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::SignatureDump {
                target: target.clone(),
                records: vec![record],
                limitations: vec!["partial".into()],
            }),
            Some(Action::SignatureDumpPartial { .. })
        ));
        let request = SignatureComparisonRequest {
            left: identity.clone(),
            right: SignatureIdentity {
                hash: Some("def".into()),
                path: Some("/tmp/busybox-old.sigdata".into()),
                ..identity
            },
        };
        let difference = SignatureDifference {
            category: SignatureDifferenceCategory::ChangedValue,
            key: "CC".into(),
            left: Some("gcc".into()),
            right: Some("clang".into()),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureComparison {
                request: request.clone(),
                differences: vec![difference.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::SignatureComparisonLoaded {
                request: request.clone(),
                differences: vec![difference],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::SignatureComparisonFailed {
                request: request.clone(),
                message: "tool failed".into(),
            }),
            Some(Action::SignatureComparisonFailed {
                request,
                message: "tool failed".into(),
            })
        );
    }

    #[test]
    fn pkgdata_model_typed_events_map_inventory_detail_partial_and_failure() {
        let inventory_request = PackageInventoryRequest { generation: 7 };
        let package = PackageSummary {
            identity: PackageIdentity::new("busybox"),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Available("/layers/core/recipes-core/busybox.bb".into()),
            version: PackageField::Available("1.37.0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Available(vec!["core-image-minimal".into()]),
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageInventory {
                request: inventory_request,
                packages: vec![package.clone()],
                limitations: Vec::new(),
            }),
            Some(Action::PackageInventoryLoaded {
                request: inventory_request,
                packages: vec![package],
            })
        );
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::PackageInventory {
                request: inventory_request,
                packages: Vec::new(),
                limitations: vec!["pkgdata directory is incomplete".into()],
            }),
            Some(Action::PackageInventoryPartial { .. })
        ));
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageInventoryFailed {
                request: inventory_request,
                message: "pkgdata directory is missing".into(),
            }),
            Some(Action::PackageInventoryFailed {
                request: inventory_request,
                message: "pkgdata directory is missing".into(),
            })
        );

        let detail_request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 3,
        };
        let detail = PackageDetail {
            identity: detail_request.identity.clone(),
            files: PackageField::Available(vec!["/bin/busybox".into()]),
            runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
            reverse_dependencies: PackageField::Unavailable,
        };
        assert!(matches!(
            model_action_from_backend_event(BackendEvent::PackageDetail {
                request: detail_request.clone(),
                detail: detail.clone(),
                limitations: vec!["reverse dependencies unavailable".into()],
            }),
            Some(Action::PackageDetailPartial { .. })
        ));
        assert_eq!(
            model_action_from_backend_event(BackendEvent::PackageDetailFailed {
                request: detail_request.clone(),
                message: "package was not found".into(),
            }),
            Some(Action::PackageDetailFailed {
                request: detail_request,
                message: "package was not found".into(),
            })
        );
    }

    #[test]
    fn image_artifact_model_typed_events_map_success_partial_and_failure() {
        let request = ImageArtifactRequest {
            generation: 9,
            machine: "qemux86-64".into(),
        };
        let artifact = ImageArtifact {
            identity: ImageArtifactIdentity {
                machine: request.machine.clone(),
                image: "core-image-minimal".into(),
                path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
            },
            kind: ImageArtifactKind::Wic,
            size_bytes: ImageArtifactField::Available(8_192),
            modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
            checksums: ImageArtifactField::Unavailable,
            manifests: ImageArtifactField::Available(Vec::new()),
            licenses: ImageArtifactField::Unavailable,
            spdx: ImageArtifactField::Unavailable,
            wic_files: ImageArtifactField::Available(Vec::new()),
        };
        let inventory = ImageArtifactInventory {
            machine: request.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: vec![artifact],
        };
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifacts {
                request: request.clone(),
                inventory: inventory.clone(),
                limitations: Vec::new(),
            }),
            Some(Action::ImageArtifactInventoryLoaded {
                request: request.clone(),
                inventory: inventory.clone(),
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifacts {
                request: request.clone(),
                inventory: inventory.clone(),
                limitations: vec!["license metadata unavailable".into()],
            }),
            Some(Action::ImageArtifactInventoryPartial {
                request: request.clone(),
                inventory,
                limitations: vec!["license metadata unavailable".into()],
            })
        );
        assert_eq!(
            model_action_from_backend_event(BackendEvent::ImageArtifactsFailed {
                request: request.clone(),
                message: "deploy directory is missing".into(),
            }),
            Some(Action::ImageArtifactInventoryFailed {
                request,
                message: "deploy directory is missing".into(),
            })
        );
    }

    #[test]
    fn image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action() {
        let request = ImageArtifactRequest {
            generation: 12,
            machine: "qemux86-64".into(),
        };
        let inventory = ImageArtifactInventory {
            machine: request.machine.clone(),
            deploy_directory: ImageArtifactField::Available(
                "/build/tmp/deploy/images/qemux86-64".into(),
            ),
            artifacts: Vec::new(),
        };
        let event: BackendEvent = yoctui_bitbake::ImageArtifactResponse {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: vec!["one symlink was not followed".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::ImageArtifactInventoryPartial {
                request,
                inventory,
                limitations: vec!["one symlink was not followed".into()],
            })
        );
    }

    #[test]
    fn pkgdata_adapter_responses_cross_the_app_boundary_as_typed_actions() {
        let inventory_request = PackageInventoryRequest { generation: 11 };
        let package = PackageSummary {
            identity: PackageIdentity::new("busybox"),
            recipe: PackageField::Available("busybox".into()),
            provider: PackageField::Unavailable,
            version: PackageField::Available("1.37.0-r0".into()),
            installed_size_bytes: PackageField::Available(1_024),
            license: PackageField::Available("GPL-2.0-only".into()),
            image_membership: PackageField::Unavailable,
        };
        let event: BackendEvent = yoctui_bitbake::PackageInventoryResponse {
            request: inventory_request,
            packages: vec![package.clone()],
            limitations: vec!["provider path unavailable".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::PackageInventoryPartial {
                request: inventory_request,
                packages: vec![package],
                limitations: vec!["provider path unavailable".into()],
            })
        );

        let request = PackageDetailRequest {
            identity: PackageIdentity::new("busybox"),
            generation: 4,
        };
        let detail = PackageDetail {
            identity: request.identity.clone(),
            files: PackageField::Available(vec!["/bin/busybox".into()]),
            runtime_dependencies: PackageField::Available(vec![PackageIdentity::new("libc6")]),
            reverse_dependencies: PackageField::Available(Vec::new()),
        };
        let event: BackendEvent = yoctui_bitbake::PackageDetailResponse {
            request: request.clone(),
            detail: detail.clone(),
            limitations: Vec::new(),
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::PackageDetailLoaded { request, detail })
        );
    }

    #[test]
    fn pkgdata_workspace_maps_search_navigation_refresh_and_context_actions() {
        assert_eq!(
            package_workspace_action(false, Input::Up),
            Some(Action::SelectPackage { delta: -1 })
        );
        assert_eq!(
            package_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedPackageDetail)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('R')),
            Some(Action::RefreshPackageInventory)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('D')),
            Some(Action::TogglePackageDependencyKind)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char(']')),
            Some(Action::SelectPackageDependency { delta: 1 })
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('d')),
            Some(Action::OpenSelectedPackageDependency)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('o')),
            Some(Action::OpenSelectedPackageRecipe)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('e')),
            Some(Action::OpenSelectedPackageProvider)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('c')),
            Some(Action::CancelPackageOperation)
        );
        assert_eq!(
            package_workspace_action(false, Input::Char('/')),
            Some(Action::BeginPackageSearch)
        );
        assert_eq!(
            package_workspace_action(true, Input::Char('b')),
            Some(Action::AppendPackageQuery('b'))
        );
        assert_eq!(
            package_workspace_action(true, Input::Backspace),
            Some(Action::BackspacePackageQuery)
        );
        assert_eq!(
            package_workspace_action(true, Input::Esc),
            Some(Action::FinishPackageSearch)
        );
        assert_eq!(package_workspace_action(false, Input::Char('x')), None);
    }

    #[test]
    fn signature_adapter_responses_cross_the_app_boundary_as_typed_actions() {
        let target = SignatureTarget {
            recipe: "busybox".into(),
            task: "do_compile".into(),
        };
        let identity = SignatureIdentity {
            target: target.clone(),
            hash: Some("aaa".into()),
            path: Some("/build/tmp/stamps/busybox/do_compile.sigdata.aaa".into()),
        };
        let record = SignatureRecord {
            identity: identity.clone(),
            base_hash: Some("base-aaa".into()),
            task_hash: Some("aaa".into()),
            variables: Vec::new(),
            dependencies: Vec::new(),
        };
        let event: BackendEvent = yoctui_bitbake::SignatureDumpResponse {
            target: target.clone(),
            records: vec![record.clone()],
            limitations: vec!["one malformed historical signature was omitted".into()],
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::SignatureDumpPartial {
                target,
                records: vec![record],
                limitations: vec!["one malformed historical signature was omitted".into()],
            })
        );

        let request = SignatureComparisonRequest {
            left: identity.clone(),
            right: SignatureIdentity {
                hash: Some("bbb".into()),
                ..identity
            },
        };
        let difference = SignatureDifference {
            category: SignatureDifferenceCategory::BaseHash,
            key: "base_hash".into(),
            left: Some("base-aaa".into()),
            right: Some("base-bbb".into()),
        };
        let event: BackendEvent = yoctui_bitbake::SignatureComparisonResponse {
            request: request.clone(),
            differences: vec![difference.clone()],
            limitations: Vec::new(),
        }
        .into();
        assert_eq!(
            model_action_from_backend_event(event),
            Some(Action::SignatureComparisonLoaded {
                request,
                differences: vec![difference],
            })
        );
    }

    #[test]
    fn typed_event_terminal_events_emit_primary_and_job_actions_once() {
        let mut coordinator = BuildJobCoordinator::default();
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap();
        let completed = coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(
            completed
                .iter()
                .filter(|action| matches!(action, Action::BuildCompleted { .. }))
                .count(),
            1
        );
        assert_eq!(
            completed
                .iter()
                .filter(|action| matches!(action, Action::SucceedBackgroundJob { .. }))
                .count(),
            1
        );
        assert_eq!(completed.len(), 2);

        let mut coordinator = BuildJobCoordinator::default();
        coordinator
            .queue_build(&request(), SystemTime::UNIX_EPOCH)
            .unwrap();
        let failed = coordinator.actions_for_backend_event(
            BackendEvent::CommandFailed {
                code: "parse".into(),
                message: "bad metadata".into(),
            },
            SystemTime::UNIX_EPOCH,
        );
        assert_eq!(
            failed
                .iter()
                .filter(|action| matches!(action, Action::Failure(_)))
                .count(),
            1
        );
        assert_eq!(
            failed
                .iter()
                .filter(|action| matches!(action, Action::FailBackgroundJob { .. }))
                .count(),
            1
        );
        assert_eq!(failed.len(), 2);
    }

    #[test]
    fn background_job_command_failure_and_disconnect_are_terminal() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let failed_id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::CommandFailed {
                    code: "start_failed".into(),
                    message: "server rejected build".into(),
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        assert_eq!(
            app.background_jobs.get(failed_id).unwrap().status,
            BackgroundJobStatus::Failed
        );

        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let lost_id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::Disconnected,
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        assert_eq!(
            app.background_jobs.get(lost_id).unwrap().status,
            BackgroundJobStatus::Lost
        );
    }

    #[test]
    fn background_job_start_failure_finishes_the_queued_job() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.start_failed(
                "executable not found".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Failed);
        assert_eq!(
            job.error.as_ref().and_then(|error| error.detail.as_deref()),
            Some("executable not found")
        );
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn background_job_backend_error_marks_the_active_job_lost() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator.backend_lost(
                "protocol framing failed".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, BackgroundJobStatus::Lost);
        assert_eq!(
            job.error.as_ref().and_then(|error| error.detail.as_deref()),
            Some("protocol framing failed")
        );
        assert_eq!(coordinator.active_job_id(), None);
    }

    #[test]
    fn background_job_cancellation_failure_recovers_then_acknowledges() {
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(10, 1_000);
        apply_actions(
            &mut app,
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .unwrap(),
        );
        let id = coordinator.active_job_id().unwrap();
        apply_actions(
            &mut app,
            coordinator
                .actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH),
        );
        assert!(matches!(
            update(&mut app, Action::Cancel),
            Some(yoctui_model::Effect::Cancel)
        ));
        apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
        apply_actions(
            &mut app,
            coordinator.cancellation_failed(
                "backend refused".into(),
                SystemTime::UNIX_EPOCH + Duration::from_secs(1),
            ),
        );
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Running
        );
        assert_eq!(app.build.status, BuildStatus::Running);
        assert!(
            app.notification
                .as_deref()
                .unwrap()
                .contains("may still be running")
        );

        assert!(matches!(
            update(&mut app, Action::Cancel),
            Some(yoctui_model::Effect::Cancel)
        ));
        apply_actions(&mut app, vec![coordinator.request_cancellation().unwrap()]);
        apply_actions(
            &mut app,
            coordinator.actions_for_backend_event(
                BackendEvent::BuildCompleted {
                    success: false,
                    exit_code: None,
                },
                SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            ),
        );
        assert_eq!(app.build.status, BuildStatus::Cancelled);
        assert_eq!(
            app.background_jobs.get(id).unwrap().status,
            BackgroundJobStatus::Cancelled
        );
    }

    #[test]
    fn background_job_coordinator_prevents_duplicate_active_builds() {
        let mut coordinator = BuildJobCoordinator::default();
        assert!(
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .is_some()
        );
        assert!(
            coordinator
                .queue_build(&request(), SystemTime::UNIX_EPOCH)
                .is_none()
        );
        assert_eq!(coordinator.active_job_id(), Some(BackgroundJobId(1)));
    }

    #[test]
    fn maps_navigation() {
        assert_eq!(
            key_action(Input::Char('l')),
            Some(Action::Open(Screen::Logs))
        );
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(key_action(Input::F5), Some(Action::OpenBuildOptions));
        assert_eq!(
            key_action(Input::Char('x')),
            Some(Action::Open(Screen::Bbmask))
        );
    }
    #[test]
    fn responsive_pane_shortcuts_map_to_focus_cycle() {
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(
            key_action(Input::BackTab),
            Some(Action::CycleFocus { backwards: true })
        );
    }
    #[test]
    fn settings_input_maps_selection_and_typed_changes() {
        assert_eq!(
            settings_action(Input::Up),
            Some(Action::SelectSetting { delta: -1 })
        );
        assert_eq!(
            settings_action(Input::Down),
            Some(Action::SelectSetting { delta: 1 })
        );
        assert_eq!(
            settings_action(Input::Left),
            Some(Action::ChangeSelectedSetting { backwards: true })
        );
        assert_eq!(
            settings_action(Input::Enter),
            Some(Action::ChangeSelectedSetting { backwards: false })
        );
        assert_eq!(
            settings_action(Input::Char('r')),
            Some(Action::RetrySettingsPersistence)
        );
        assert_eq!(settings_action(Input::Esc), None);
    }
    #[test]
    fn build_environment_input_verifies_and_returns_to_dashboard() {
        assert_eq!(
            build_environment_action(Input::Char('V')),
            Some(Action::BeginBuildEnvironmentVerification)
        );
        assert_eq!(
            build_environment_action(Input::Esc),
            Some(Action::Open(Screen::Dashboard))
        );
    }
    #[test]
    fn popup_editor_input_maps_normal_and_insert_modes_without_leakage() {
        assert_eq!(
            popup_editor_action(false, Input::Char('e')),
            Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
        );
        assert_eq!(
            popup_editor_action(false, Input::Char('j')),
            Some(Action::EditActivePopup(PopupEditorCommand::Down))
        );
        assert_eq!(popup_editor_action(false, Input::Char('x')), None);
        assert_eq!(
            popup_editor_action(true, Input::Char('k')),
            Some(Action::EditActivePopup(PopupEditorCommand::Insert('k')))
        );
        assert_eq!(
            popup_editor_action(true, Input::Home),
            Some(Action::EditActivePopup(PopupEditorCommand::Home))
        );
        assert_eq!(
            popup_editor_action(true, Input::CtrlV),
            Some(Action::EditActivePopup(PopupEditorCommand::Paste))
        );
        assert_eq!(popup_editor_action(true, Input::Enter), None);
    }
    #[test]
    fn live_tasks_input_maps_selection_and_filter_controls() {
        assert_eq!(
            tasks_action(false, Input::Down),
            Some(Action::ScrollBuildTasks { delta: 1 })
        );
        assert_eq!(
            tasks_action(false, Input::Char('f')),
            Some(Action::CycleTaskStateFilter)
        );
        assert_eq!(
            tasks_action(false, Input::Char('F')),
            Some(Action::CycleTaskFilterField)
        );
        assert_eq!(
            tasks_action(false, Input::Char('/')),
            Some(Action::BeginTaskFilterEdit)
        );
        assert_eq!(
            tasks_action(true, Input::Char('x')),
            Some(Action::AppendTaskFilter('x'))
        );
        assert_eq!(
            tasks_action(true, Input::Esc),
            Some(Action::FinishTaskFilterEdit)
        );
        let action = model_action_from_backend_event(BackendEvent::TaskQueued {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            worker: Some("worker-1".into()),
            stats: Some(yoctui_model::TaskStats {
                completed: 3,
                total: 10,
                active: 2,
                failed: 0,
            }),
        });
        assert!(matches!(
            action,
            Some(Action::TaskQueued(TaskInfo {
                worker: Some(worker),
                stats: Some(yoctui_model::TaskStats { total: 10, .. }),
                ..
            })) if worker == "worker-1"
        ));
    }
    #[test]
    fn config_metadata_normalizes_typed_scope_and_history_once() {
        let action = model_action_from_backend_event(BackendEvent::Variable {
            name: "PACKAGE_ARCH".into(),
            recipe: Some("base-files".into()),
            value: Some("qemux86_64".into()),
            provenance: Some("/layers/meta/conf/machine/qemux86-64.conf:5".into()),
            unexpanded_value: Some("${MACHINE_ARCH}".into()),
            operations: vec![yoctui_model::VariableOperation {
                operation: "set".into(),
                file: Some("/layers/meta/conf/machine/qemux86-64.conf".into()),
                line: Some(5),
                value: Some("${MACHINE_ARCH}".into()),
            }],
            active_overrides: vec!["qemux86-64".into()],
        });
        assert!(matches!(
            action,
            Some(Action::VariableLoaded(VariableDetail {
                identity: VariableIdentity {
                    name,
                    recipe: Some(recipe),
                },
                unexpanded_value: Some(unexpanded),
                operations,
                ..
            })) if name == "PACKAGE_ARCH"
                && recipe == "base-files"
                && unexpanded == "${MACHINE_ARCH}"
                && operations.len() == 1
        ));
    }
    #[test]
    fn command_palette_global_shortcut_is_typed() {
        assert_eq!(key_action(Input::CtrlP), Some(Action::OpenCommandPalette));
        assert_eq!(focus_action(FocusTarget::CommandPalette, Input::Tab), None);
    }
    #[test]
    fn dialog_focus_navigation_keys_are_typed_before_cli_routing() {
        assert_eq!(
            key_action(Input::Tab),
            Some(Action::CycleFocus { backwards: false })
        );
        assert_eq!(
            key_action(Input::BackTab),
            Some(Action::CycleFocus { backwards: true })
        );
        assert_eq!(
            key_action(Input::Esc),
            Some(Action::Open(Screen::Dashboard))
        );
        assert_eq!(
            focus_action(FocusTarget::Navigator, Input::Up),
            Some(Action::SelectNavigator { delta: -1 })
        );
        assert_eq!(
            focus_action(FocusTarget::Inspector, Input::Up),
            None,
            "inspector arrows must not leak into workspace actions"
        );
        assert_eq!(
            focus_action(FocusTarget::Dialog, Input::Tab),
            None,
            "modal input is handled only by the active dialog"
        );
    }
    #[test]
    fn maps_log_controls() {
        assert_eq!(key_action(Input::Char('f')), Some(Action::ToggleLogFollow));
        assert_eq!(key_action(Input::Char('w')), Some(Action::ToggleLogWrap));
        assert_eq!(key_action(Input::Up), Some(Action::ScrollLogs { delta: 1 }));
    }
    #[test]
    fn log_workspace_maps_selection_search_filters_and_selected_actions() {
        assert_eq!(
            logs_action(false, Input::Up),
            Some(Action::ScrollLogs { delta: 1 })
        );
        assert_eq!(
            logs_action(false, Input::Char('B')),
            Some(Action::CycleLogBuildFilter)
        );
        assert_eq!(
            logs_action(false, Input::Char('o')),
            Some(Action::OpenSelectedLogSource)
        );
        assert_eq!(
            logs_action(false, Input::Char('C')),
            Some(Action::CopySelectedLog)
        );
        assert_eq!(
            logs_action(true, Input::Char('x')),
            Some(Action::AppendLogQuery('x'))
        );
        assert_eq!(logs_action(true, Input::Esc), Some(Action::FinishLogSearch));
    }
    #[test]
    fn enter_activates_contextual_notification() {
        assert_eq!(key_action(Input::Enter), Some(Action::ActivateNotification));
    }
    #[test]
    fn maps_severity_filter_control() {
        assert_eq!(key_action(Input::Char('s')), Some(Action::CycleLogSeverity));
    }
    #[test]
    fn error_workspace_maps_selection_log_jump_and_source_open() {
        assert_eq!(
            errors_action(Input::Up),
            Some(Action::SelectError { delta: -1 })
        );
        assert_eq!(
            errors_action(Input::Enter),
            Some(Action::JumpToSelectedError)
        );
        assert_eq!(
            errors_action(Input::Char('o')),
            Some(Action::OpenSelectedErrorSource)
        );
    }
    #[test]
    fn layer_tree_maps_lazy_navigation_hidden_refresh_and_inspector_modes() {
        assert_eq!(
            layer_tree_action(false, Input::Right),
            Some(Action::LayerBrowserExpand)
        );
        assert_eq!(
            layer_tree_action(false, Input::Left),
            Some(Action::LayerBrowserUp)
        );
        assert_eq!(
            layer_tree_action(false, Input::Char('.')),
            Some(Action::ToggleLayerBrowserHidden)
        );
        assert_eq!(
            layer_tree_action(false, Input::Char('g')),
            Some(Action::SetLayerInspectorMode(LayerInspectorMode::Git))
        );
        assert_eq!(
            layer_tree_action(true, Input::Char('b')),
            Some(Action::AppendMetadataQuery('b'))
        );
    }
    #[test]
    fn recipes_workspace_maps_search_selection_detail_and_dependencies() {
        assert_eq!(
            recipes_workspace_action(false, Input::Down),
            Some(Action::SelectRecipe { delta: 1 })
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedRecipeMetadata)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('g')),
            Some(Action::BeginSelectedRecipeDependencies)
        );
        assert_eq!(
            recipes_workspace_action(true, Input::Char('b')),
            Some(Action::AppendMetadataQuery('b'))
        );
        assert_eq!(
            recipes_workspace_action(true, Input::Backspace),
            Some(Action::BackspaceMetadataQuery)
        );
    }
    #[test]
    fn dependency_workspace_maps_typed_navigation_refresh_and_open_actions() {
        assert_eq!(
            dependency_workspace_action(Input::Up),
            Some(Action::SelectDependencyGraphNode { delta: -1 })
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('j')),
            Some(Action::SelectDependencyGraphNode { delta: 1 })
        );
        assert_eq!(
            dependency_workspace_action(Input::Enter),
            Some(Action::OpenSelectedDependencyRecipe)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('o')),
            Some(Action::OpenSelectedDependencyProvider)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('L')),
            Some(Action::OpenSelectedDependencyTaskLog)
        );
        assert_eq!(
            dependency_workspace_action(Input::Char('r')),
            Some(Action::RefreshDependencyGraph)
        );
        assert_eq!(dependency_workspace_action(Input::Char('x')), None);
    }
    #[test]
    fn recipe_bitbake_action_maps_standard_and_forced_task_controls() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('f')),
            Some(Action::BeginSelectedRecipeForceTask)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('v')),
            Some(Action::BeginSelectedRecipeDevshell)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('K')),
            Some(Action::BeginSelectedRecipeDiffconfig)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('z')),
            Some(Action::BeginSelectedRecipeDiffsigs)
        );
        let request = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("compile".into()),
            force: true,
        };
        let mut coordinator = BuildJobCoordinator::default();
        let actions = coordinator
            .queue_build(&request, SystemTime::UNIX_EPOCH)
            .unwrap();
        assert!(matches!(
            &actions[0],
            Action::QueueBackgroundJob(spec)
                if spec.context.target.as_deref() == Some("busybox")
                    && spec.context.task.as_deref() == Some("compile")
        ));
        assert!(
            coordinator
                .queue_build(&request, SystemTime::UNIX_EPOCH)
                .is_none()
        );
    }
    #[test]
    fn recipe_navigation_maps_files_logs_patches_and_devtool_routes() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('e')),
            Some(Action::OpenSelectedRecipeProvider)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('o')),
            Some(Action::BeginSelectedRecipeTaskLog)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('p')),
            Some(Action::BeginSelectedRecipePatchReview)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('d')),
            Some(Action::BeginSelectedRecipeDevtoolModify)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('t')),
            Some(Action::BeginSelectedRecipeDevtoolStatus)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('u')),
            Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('F')),
            Some(Action::BeginSelectedRecipeDevtoolFinish)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('P')),
            Some(Action::BeginSelectedRecipeDevtoolDeploy)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('D')),
            Some(Action::BeginSelectedRecipeDevtoolReset)
        );
    }

    #[test]
    fn devtool_metadata_shortcut_requests_typed_status() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('t')),
            Some(Action::BeginSelectedRecipeDevtoolStatus)
        );
    }

    #[test]
    fn config_workspace_maps_search_selection_and_lazy_detail() {
        assert_eq!(
            config_workspace_action(false, Input::Down),
            Some(Action::SelectConfigVariable { delta: 1 })
        );
        assert_eq!(
            config_workspace_action(false, Input::Enter),
            Some(Action::BeginSelectedConfigDetail)
        );
        assert_eq!(
            config_workspace_action(false, Input::Char('/')),
            Some(Action::BeginMetadataSearch)
        );
        assert_eq!(
            config_workspace_action(true, Input::Char('M')),
            Some(Action::AppendMetadataQuery('M'))
        );
    }

    #[test]
    fn config_copy_shortcuts_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('C')),
            Some(Action::CopySelectedConfigEffective)
        );
        assert_eq!(
            config_workspace_action(false, Input::Char('U')),
            Some(Action::CopySelectedConfigUnexpanded)
        );
    }

    #[test]
    fn config_source_picker_keys_are_modal_and_typed() {
        assert_eq!(
            config_source_picker_action(Input::Down),
            Some(Action::SelectConfigSource { delta: 1 })
        );
        assert_eq!(
            config_source_picker_action(Input::Enter),
            Some(Action::OpenSelectedConfigSourceChoice)
        );
        assert_eq!(
            config_source_picker_action(Input::Esc),
            Some(Action::CancelConfigSourcePicker)
        );
    }

    #[test]
    fn config_scope_shortcut_and_picker_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('s')),
            Some(Action::OpenConfigScopePicker)
        );
        assert_eq!(
            config_scope_picker_action(Input::Down),
            Some(Action::SelectConfigScope { delta: 1 })
        );
        assert_eq!(
            config_scope_picker_action(Input::Enter),
            Some(Action::ConfirmConfigScope)
        );
        assert_eq!(
            config_scope_picker_action(Input::Esc),
            Some(Action::CancelConfigScopePicker)
        );
    }

    #[test]
    fn config_compare_shortcut_and_close_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('c')),
            Some(Action::OpenConfigComparison)
        );
        assert_eq!(
            config_compare_dialog_action(Input::Enter),
            Some(Action::CloseConfigComparison)
        );
        assert_eq!(
            config_compare_dialog_action(Input::Esc),
            Some(Action::CloseConfigComparison)
        );
    }

    #[test]
    fn config_edit_preview_shortcut_and_dialog_keys_are_typed() {
        assert_eq!(
            config_workspace_action(false, Input::Char('E')),
            Some(Action::BeginConfigEdit)
        );
        assert_eq!(
            config_edit_dialog_action(Input::Char('x')),
            Some(Action::AppendConfigEdit('x'))
        );
        assert_eq!(
            config_edit_dialog_action(Input::Enter),
            Some(Action::PreviewConfigEdit)
        );
        assert_eq!(
            config_edit_confirmation_action(Input::Enter),
            Some(Action::ConfirmConfigEdit)
        );
    }

    #[test]
    fn config_edit_write_confirmation_is_modal_and_cancellable() {
        assert_eq!(
            config_edit_confirmation_action(Input::Enter),
            Some(Action::ConfirmConfigEdit)
        );
        assert_eq!(
            config_edit_confirmation_action(Input::Esc),
            Some(Action::CancelConfigEditConfirmation)
        );
        assert_eq!(config_edit_confirmation_action(Input::Char('E')), None);
    }

    #[test]
    fn devtool_modify_routes_confirmation_and_workspace_editor_build_keys() {
        assert_eq!(
            devtool_modify_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolModify)
        );
        assert_eq!(
            devtool_modify_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolModify)
        );
        assert_eq!(devtool_modify_confirmation_action(Input::Char('b')), None);
        assert_eq!(
            recipe_editor_action(false, Input::CtrlB),
            Some(Action::BeginRecipeEditorBuild)
        );
        assert_eq!(
            recipe_editor_action(false, Input::Enter),
            Some(Action::ToggleRecipeEditorEditing)
        );
        assert_eq!(
            recipe_editor_action(true, Input::Enter),
            Some(Action::AppendRecipeEditor('\n'))
        );
    }

    #[test]
    fn devtool_publish_update_routes_only_confirmation_keys() {
        assert_eq!(
            devtool_update_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolUpdateRecipe)
        );
        assert_eq!(
            devtool_update_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolUpdateRecipe)
        );
        assert_eq!(devtool_update_confirmation_action(Input::Char('u')), None);
    }

    #[test]
    fn devtool_publish_finish_routes_picker_and_confirmation_keys() {
        assert_eq!(
            devtool_finish_picker_action(Input::Up),
            Some(Action::SelectDevtoolFinishLayer { delta: -1 })
        );
        assert_eq!(
            devtool_finish_picker_action(Input::Down),
            Some(Action::SelectDevtoolFinishLayer { delta: 1 })
        );
        assert_eq!(
            devtool_finish_picker_action(Input::Enter),
            Some(Action::PreviewDevtoolFinish)
        );
        assert_eq!(
            devtool_finish_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolFinish)
        );
        assert_eq!(
            devtool_finish_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolFinishConfirmation)
        );
    }

    #[test]
    fn devtool_target_deploy_routes_entry_and_confirmation_keys() {
        assert_eq!(
            devtool_deploy_dialog_action(Input::Char('q')),
            Some(Action::AppendDevtoolDeployTarget('q'))
        );
        assert_eq!(
            devtool_deploy_dialog_action(Input::Backspace),
            Some(Action::BackspaceDevtoolDeployTarget)
        );
        assert_eq!(
            devtool_deploy_dialog_action(Input::Enter),
            Some(Action::PreviewDevtoolDeploy)
        );
        assert_eq!(
            devtool_deploy_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolDeploy)
        );
        assert_eq!(
            devtool_deploy_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolDeployConfirmation)
        );
    }

    #[test]
    fn devtool_target_reset_routes_only_destructive_confirmation_keys() {
        assert_eq!(
            devtool_reset_confirmation_action(Input::Enter),
            Some(Action::ConfirmDevtoolReset)
        );
        assert_eq!(
            devtool_reset_confirmation_action(Input::Esc),
            Some(Action::CancelDevtoolReset)
        );
        assert_eq!(devtool_reset_confirmation_action(Input::Char('D')), None);
    }

    #[test]
    fn devtool_job_lifecycle_maps_runner_events_and_stays_independent_from_bitbake() {
        let now = SystemTime::UNIX_EPOCH;
        let mut devtool = DevtoolJobCoordinator::default();
        let operation = DevtoolOperation::Reset {
            recipe: "busybox".into(),
        };
        let actions = devtool.queue(operation.clone(), now).unwrap();
        let id = devtool.active_job_id().unwrap();
        assert_eq!(id, BackgroundJobId(1_u64 << 63));
        assert_eq!(devtool.active_operation(), Some(&operation));
        assert!(devtool.queue(operation, now).is_none());

        let mut build = BuildJobCoordinator::default();
        let build_actions = build
            .queue_build(
                &BuildRequest {
                    targets: vec!["core-image-minimal".into()],
                    task: None,
                    force: false,
                },
                now,
            )
            .unwrap();
        assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
        assert_ne!(build.active_job_id(), devtool.active_job_id());

        let mut app = yoctui_model::App::new(10, 1_000);
        for action in actions.into_iter().chain(build_actions) {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in devtool.actions_for_event(DevtoolRunnerEvent::Started, now) {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in devtool.actions_for_event(
            DevtoolRunnerEvent::Output {
                stream: DevtoolOutputStream::Stderr,
                line: "progress".into(),
                truncated: true,
            },
            now,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        app.screen = Screen::Dashboard;
        for action in
            devtool.actions_for_event(DevtoolRunnerEvent::Completed { exit_code: Some(0) }, now)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        let job = app.background_jobs.get(id).unwrap();
        assert_eq!(job.status, yoctui_model::BackgroundJobStatus::Succeeded);
        assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
        assert!(job.output[0].truncated);
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(build.active_job_id(), Some(BackgroundJobId(1)));
    }

    #[test]
    fn devtool_job_lifecycle_maps_start_failure_cancel_failure_cancel_and_loss() {
        let now = SystemTime::UNIX_EPOCH;
        let operation = DevtoolOperation::Reset {
            recipe: "busybox".into(),
        };

        let mut coordinator = DevtoolJobCoordinator::default();
        let id = {
            let _ = coordinator.queue(operation.clone(), now);
            coordinator.active_job_id().unwrap()
        };
        assert!(matches!(
            coordinator.start_failed("missing".into(), now).as_slice(),
            [Action::FailBackgroundJob { id: failed, .. }] if *failed == id
        ));

        let mut coordinator = DevtoolJobCoordinator::default();
        let _ = coordinator.queue(operation.clone(), now);
        assert!(matches!(
            coordinator.request_cancellation(),
            Some(Action::RequestBackgroundJobCancellation { .. })
        ));
        assert!(coordinator.request_cancellation().is_none());
        let rejected = coordinator.cancellation_failed("signal".into(), now);
        assert!(matches!(
            rejected.last(),
            Some(Action::RejectBackgroundJobCancellation { .. })
        ));
        assert!(matches!(
            coordinator
                .actions_for_event(
                    DevtoolRunnerEvent::Cancelled {
                        forced: true,
                        exit_code: None,
                    },
                    now,
                )
                .last(),
            Some(Action::CancelBackgroundJob { .. })
        ));

        let mut coordinator = DevtoolJobCoordinator::default();
        let _ = coordinator.queue(operation, now);
        assert!(matches!(
            coordinator
                .actions_for_event(
                    DevtoolRunnerEvent::Lost {
                        message: "channel".into(),
                    },
                    now,
                )
                .as_slice(),
            [Action::LoseBackgroundJob { .. }]
        ));
    }

    #[test]
    fn recipe_qa_action_maps_capabilities_and_persists_terminal_job_outcomes() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('V')),
            Some(Action::BeginSelectedRecipeCveCheck)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('X')),
            Some(Action::BeginSelectedRecipeSpdx)
        );

        let cve = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cve_check".into()),
            force: false,
        };
        let mut coordinator = BuildJobCoordinator::default();
        let mut app = App::new(20, 4_000);
        let queued = coordinator
            .queue_build(&cve, SystemTime::UNIX_EPOCH)
            .unwrap();
        assert!(matches!(
            &queued[0],
            Action::QueueBackgroundJob(spec)
                if spec.kind == BackgroundJobKind::CveCheck
                    && spec.context.workspace == Some(Screen::Recipes)
                    && spec.context.recipe.as_deref() == Some("busybox")
                    && spec.context.task.as_deref() == Some("cve_check")
        ));
        for action in queued {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator
            .actions_for_backend_event(BackendEvent::BuildStarted, SystemTime::UNIX_EPOCH)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: true,
                exit_code: Some(0),
            },
            SystemTime::UNIX_EPOCH,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        let cve_job = app.background_jobs.jobs.back().unwrap();
        assert_eq!(cve_job.status, BackgroundJobStatus::Succeeded);
        assert!(
            cve_job
                .result
                .as_ref()
                .unwrap()
                .summary
                .contains("no result path")
        );
        assert!(cve_job.result.as_ref().unwrap().artifacts.is_empty());

        let spdx = BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("create_spdx".into()),
            force: false,
        };
        for action in coordinator
            .queue_build(&spdx, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert!(
            coordinator
                .queue_build(&spdx, SystemTime::UNIX_EPOCH)
                .is_none()
        );
        let cancellation = coordinator.request_cancellation().unwrap();
        let _ = yoctui_model::update(&mut app, cancellation);
        for action in coordinator.actions_for_backend_event(
            BackendEvent::BuildCompleted {
                success: false,
                exit_code: Some(130),
            },
            SystemTime::UNIX_EPOCH,
        ) {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert_eq!(
            app.background_jobs.jobs.back().unwrap().status,
            BackgroundJobStatus::Cancelled
        );

        for action in coordinator
            .queue_build(&cve, SystemTime::UNIX_EPOCH)
            .unwrap()
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        for action in coordinator
            .actions_for_backend_event(BackendEvent::Disconnected, SystemTime::UNIX_EPOCH)
        {
            let _ = yoctui_model::update(&mut app, action);
        }
        assert_eq!(
            app.background_jobs.jobs.back().unwrap().status,
            BackgroundJobStatus::Lost
        );
    }

    #[test]
    fn signature_workspace_maps_recipe_entry_picker_and_workspace_keys() {
        assert_eq!(
            recipes_workspace_action(false, Input::Char('Z')),
            Some(Action::BeginSelectedRecipeSignatures)
        );
        assert_eq!(
            recipes_workspace_action(false, Input::Char('z')),
            Some(Action::BeginSelectedRecipeDiffsigs)
        );
        assert_eq!(
            signature_task_picker_action(Input::Down),
            Some(Action::SelectSignatureTask { delta: 1 })
        );
        assert_eq!(
            signature_task_picker_action(Input::Enter),
            Some(Action::ConfirmSignatureTask)
        );
        assert_eq!(
            signature_task_picker_action(Input::Esc),
            Some(Action::CancelSignatureTaskPicker)
        );
        assert_eq!(
            signature_workspace_action(Input::Up),
            Some(Action::SelectSignatureRecord { delta: -1 })
        );
        assert_eq!(
            signature_workspace_action(Input::Char('1')),
            Some(Action::SetSelectedSignatureComparisonSide(
                yoctui_model::SignatureComparisonSide::Left
            ))
        );
        assert_eq!(
            signature_workspace_action(Input::Char('2')),
            Some(Action::SetSelectedSignatureComparisonSide(
                yoctui_model::SignatureComparisonSide::Right
            ))
        );
        assert_eq!(
            signature_workspace_action(Input::Char('c')),
            Some(Action::BeginSignatureComparison)
        );
        assert_eq!(
            signature_workspace_action(Input::Char('r')),
            Some(Action::RefreshSignatureDump)
        );
        assert_eq!(
            signature_workspace_action(Input::Char('e')),
            Some(Action::OpenSignatureProvider)
        );
        assert_eq!(
            signature_workspace_action(Input::Esc),
            Some(Action::LeaveSignatureWorkspace)
        );
        assert_eq!(signature_workspace_action(Input::Char('x')), None);
    }
    #[test]
    fn images_workspace_image_action_maps_search_refresh_build_cancel_and_open_actions() {
        assert_eq!(
            images_workspace_action(false, Input::Up),
            Some(Action::SelectImageArtifact { delta: -1 })
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('R')),
            Some(Action::RefreshImageArtifactInventory)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('b')),
            Some(Action::BeginSelectedImageArtifactBuild)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('c')),
            Some(Action::CancelImageArtifactOperation)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('o')),
            Some(Action::OpenSelectedImageArtifact)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('m')),
            Some(Action::OpenSelectedImageArtifactAssociation(
                yoctui_model::ImageArtifactAssociation::Manifest
            ))
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('/')),
            Some(Action::BeginImageArtifactSearch)
        );
        assert_eq!(
            images_workspace_action(true, Input::Char('w')),
            Some(Action::AppendImageArtifactQuery('w'))
        );
        assert_eq!(
            images_workspace_action(true, Input::Esc),
            Some(Action::FinishImageArtifactSearch)
        );
    }
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
    #[test]
    fn qemu_workspace_maps_launch_edit_preview_and_cancellation_keys() {
        assert_eq!(
            images_workspace_action(false, Input::Char('Q')),
            Some(Action::BeginSelectedQemuLaunch)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('x')),
            Some(Action::BeginActiveImageRuntimeCancellation)
        );
        assert_eq!(
            qemu_launch_dialog_action(false, Input::Down),
            Some(Action::SelectQemuLaunchField { delta: 1 })
        );
        assert_eq!(
            qemu_launch_dialog_action(false, Input::Left),
            Some(Action::CycleQemuLaunchChoice { backwards: true })
        );
        assert_eq!(
            qemu_launch_dialog_action(false, Input::Enter),
            Some(Action::ActivateQemuLaunchField)
        );
        assert_eq!(
            qemu_launch_dialog_action(true, Input::Char('/')),
            Some(Action::AppendQemuLaunchField('/'))
        );
        assert_eq!(
            qemu_launch_dialog_action(true, Input::Backspace),
            Some(Action::BackspaceQemuLaunchField)
        );
        assert_eq!(
            qemu_launch_dialog_action(true, Input::Enter),
            Some(Action::FinishQemuLaunchFieldEdit)
        );
        assert_eq!(
            qemu_launch_dialog_action(false, Input::Char('p')),
            Some(Action::PreviewQemuLaunch)
        );
        assert_eq!(
            qemu_launch_dialog_action(true, Input::Esc),
            Some(Action::CancelQemuLaunch)
        );
        assert_eq!(
            qemu_launch_confirmation_action(Input::Enter),
            Some(Action::ConfirmQemuLaunch)
        );
        assert_eq!(
            qemu_launch_confirmation_action(Input::Esc),
            Some(Action::CancelQemuLaunchPreview)
        );
        assert_eq!(
            qemu_cancellation_confirmation_action(Input::Enter),
            Some(Action::ConfirmQemuSessionCancellation)
        );
        assert_eq!(
            qemu_cancellation_confirmation_action(Input::Esc),
            Some(Action::CancelQemuSessionCancellation)
        );
    }
    #[test]
    fn wic_device_write_and_creation_map_distinct_modal_and_artifact_keys() {
        assert_eq!(
            images_workspace_action(false, Input::Char('W')),
            Some(Action::BeginSelectedWicCreate)
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('w')),
            Some(Action::OpenSelectedImageArtifactAssociation(
                yoctui_model::ImageArtifactAssociation::Wic
            ))
        );
        assert_eq!(
            wic_create_dialog_action(false, Input::Down),
            Some(Action::SelectWicCreateField { delta: 1 })
        );
        assert_eq!(
            wic_create_dialog_action(false, Input::Left),
            Some(Action::CycleWicCreateChoice { backwards: true })
        );
        assert_eq!(
            wic_create_dialog_action(true, Input::Char('/')),
            Some(Action::AppendWicCreateField('/'))
        );
        assert_eq!(
            wic_create_dialog_action(false, Input::Char('p')),
            Some(Action::PreviewWicCreate)
        );
        assert_eq!(
            wic_create_confirmation_action(Input::Enter),
            Some(Action::ConfirmWicCreate)
        );
        assert_eq!(
            wic_cancellation_confirmation_action(WicSessionId(3), false, Input::Enter),
            Some(Action::ConfirmWicSessionCancellation {
                id: WicSessionId(3),
                acknowledge_incomplete_device: false,
            })
        );
        assert_eq!(
            images_workspace_action(false, Input::Char('D')),
            Some(Action::BeginSelectedWicDeviceWrite)
        );
        assert_eq!(
            wic_device_picker_action(Input::Down),
            Some(Action::SelectWicDevice { delta: 1 })
        );
        assert_eq!(
            wic_device_picker_action(Input::Enter),
            Some(Action::ConfirmWicDeviceSelection)
        );
        assert_eq!(
            wic_write_phrase_action(Input::Char('W')),
            Some(Action::AppendWicWritePhrase('W'))
        );
        assert_eq!(
            wic_write_phrase_action(Input::Enter),
            Some(Action::PreviewWicDeviceWrite)
        );
        assert_eq!(
            wic_write_confirmation_action(Input::Enter),
            Some(Action::ConfirmWicDeviceWrite)
        );
        assert_eq!(
            wic_cancellation_confirmation_action(WicSessionId(4), true, Input::Enter),
            Some(Action::ConfirmWicSessionCancellation {
                id: WicSessionId(4),
                acknowledge_incomplete_device: true,
            })
        );
    }
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
    #[test]
    fn wic_adapter_capability_crosses_app_boundary_without_parsing() {
        let capability = WicCapability::MissingKickstarts {
            executable: "/usr/bin/wic".into(),
        };
        assert_eq!(
            wic_capability_action(capability.clone()),
            Action::WicCapabilityLoaded(capability)
        );
    }
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
    #[test]
    fn wic_device_write_adapter_boundary_preserves_inventory_and_loss() {
        let request = yoctui_model::WicDeviceInventoryRequest {
            generation: 3,
            image: yoctui_model::WicOutputIdentity {
                path: "/build/output/image.wic".into(),
                size_bytes: 4_096,
                modified_unix_seconds: 7,
            },
        };
        let device = yoctui_model::WicDevice {
            identity: yoctui_model::WicDeviceIdentity {
                path: "/dev/sdz".into(),
                major_minor: "8:240".into(),
                size_bytes: 8_192,
                model: Some("fixture".into()),
                serial: Some("serial".into()),
                transport: Some("usb".into()),
            },
            removable: true,
            writable: true,
            read_only: false,
            descendant_mounts: Vec::new(),
            unavailable_reason: None,
        };
        assert_eq!(
            wic_device_inventory_action(WicDeviceInventoryResponse {
                request: request.clone(),
                devices: vec![device.clone()],
                limitations: vec!["excluded /dev/sda".into()],
            }),
            Action::WicDeviceInventoryLoaded {
                request,
                devices: vec![device],
                limitations: vec!["excluded /dev/sda".into()],
            }
        );
        let id = WicSessionId(11);
        assert_eq!(
            wic_actions_for_runner_event(
                id,
                WicRunnerEvent::Lost {
                    message: "write output channel lost".into(),
                },
                SystemTime::UNIX_EPOCH,
            ),
            vec![Action::LoseWicSession {
                id,
                message: "write output channel lost".into(),
                finished_at: SystemTime::UNIX_EPOCH,
            }]
        );
    }
    #[test]
    fn maps_recipe_and_task_filter_controls() {
        assert_eq!(
            key_action(Input::Char('R')),
            Some(Action::CycleLogRecipeFilter)
        );
        assert_eq!(
            key_action(Input::Char('T')),
            Some(Action::CycleLogTaskFilter)
        );
    }
    #[test]
    fn maps_log_match_navigation_controls() {
        assert_eq!(key_action(Input::Char('n')), Some(Action::NextLogMatch));
        assert_eq!(key_action(Input::Char('N')), Some(Action::PreviousLogMatch));
    }

    #[test]
    fn sdk_workflow_maps_workspace_and_modal_keys_without_leakage() {
        assert_eq!(
            sdk_workspace_action(false, Input::Char('s')),
            Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
                SdkKind::Standard
            )))
        );
        assert_eq!(
            sdk_workspace_action(false, Input::Char('E')),
            Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
                SdkKind::Extensible
            )))
        );
        assert_eq!(
            sdk_workspace_action(false, Input::Char('t')),
            Some(Action::BeginSdkBuild(SdkBuildAction::Test(
                SdkKind::Standard
            )))
        );
        assert_eq!(
            sdk_workspace_action(false, Input::Char('T')),
            Some(Action::BeginSdkBuild(SdkBuildAction::Test(
                SdkKind::Extensible
            )))
        );
        assert_eq!(
            sdk_workspace_action(true, Input::Char('x')),
            Some(Action::AppendSdkArtifactQuery('x'))
        );
        assert_eq!(
            sdk_workspace_action(false, Input::Char('o')),
            Some(Action::OpenSelectedSdkArtifact)
        );
        assert_eq!(
            sdk_build_confirmation_action(Input::Enter),
            Some(Action::ConfirmSdkBuild)
        );
        assert_eq!(
            sdk_publish_dialog_action(Input::Char('P')),
            Some(Action::AppendSdkPublishDestination('P')),
            "publication modal input must not leak to the SDK workspace"
        );
        assert_eq!(
            sdk_publish_confirmation_action(Input::Enter),
            Some(Action::ConfirmSdkPublish)
        );
        assert_eq!(
            sdk_native_dialog_action(false, Input::Esc),
            Some(Action::CancelSdkNative)
        );
        assert_eq!(
            sdk_native_dialog_action(false, Input::Char('p')),
            Some(Action::PreviewSdkNative)
        );
        assert_eq!(
            sdk_native_dialog_action(false, Input::Down),
            Some(Action::SelectSdkNativeField { delta: 1 })
        );
        assert_eq!(
            sdk_native_dialog_action(false, Input::Enter),
            Some(Action::ActivateSdkNativeField)
        );
        assert_eq!(
            sdk_native_dialog_action(true, Input::Char('x')),
            Some(Action::AppendSdkNativeField('x'))
        );
        assert_eq!(
            sdk_native_dialog_action(true, Input::Enter),
            Some(Action::FinishSdkNativeFieldEdit)
        );
        assert_eq!(
            sdk_native_dialog_action(true, Input::Esc),
            Some(Action::CancelSdkNative),
            "Esc closes the dialog even while a field is being edited"
        );
        assert_eq!(
            sdk_cancellation_confirmation_action(Input::Enter),
            Some(Action::ConfirmSdkSessionCancellation)
        );
        let id = SdkSessionId(7);
        assert_eq!(
            sdk_actions_for_runner_event(id, SdkToolRunnerEvent::Started, SystemTime::UNIX_EPOCH),
            vec![
                Action::SdkSessionStarting {
                    id,
                    started_at: SystemTime::UNIX_EPOCH
                },
                Action::SdkSessionRunning { id }
            ]
        );
        assert!(matches!(
            sdk_actions_for_runner_event(
                id,
                SdkToolRunnerEvent::TimedOut {
                    forced: true,
                    exit_code: None
                },
                SystemTime::UNIX_EPOCH
            )
            .as_slice(),
            [Action::FailSdkSession {
                id: SdkSessionId(7),
                message,
                exit_code: None,
                ..
            }] if message.contains("timed out") && message.contains("forced")
        ));
    }

    #[test]
    fn test_workflow_test_runner_maps_keys_and_classifies_managed_bitbake_tests() {
        assert_eq!(
            testing_workspace_action(Input::Down),
            Some(Action::SelectTestFamily { delta: 1 })
        );
        assert_eq!(
            testing_workspace_action(Input::Char('r')),
            Some(Action::BeginSelectedTestLaunch)
        );
        assert_eq!(
            testing_workspace_action(Input::Char('x')),
            Some(Action::BeginActiveTestSessionCancellation)
        );
        assert_eq!(
            test_launch_dialog_action(false, Input::Down),
            Some(Action::SelectTestLaunchField { delta: 1 })
        );
        assert_eq!(
            test_launch_dialog_action(false, Input::Char('p')),
            Some(Action::PreviewTestLaunch)
        );
        assert_eq!(
            test_launch_dialog_action(true, Input::Char('x')),
            Some(Action::AppendTestLaunchField('x')),
            "editable launch fields must trap printable input"
        );
        assert_eq!(
            test_launch_dialog_action(true, Input::Enter),
            Some(Action::FinishTestLaunchFieldEdit)
        );
        assert_eq!(
            test_launch_dialog_action(true, Input::Esc),
            Some(Action::CancelTestLaunch),
            "Esc closes the launch dialog while a field is edited"
        );
        assert_eq!(
            test_launch_confirmation_action(Input::Enter),
            Some(Action::ConfirmTestLaunch)
        );
        assert_eq!(
            test_cancellation_confirmation_action(Input::Enter),
            Some(Action::ConfirmTestSessionCancellation)
        );

        let mut coordinator = BuildJobCoordinator::default();
        let actions = coordinator
            .queue_build(
                &BuildRequest {
                    targets: vec!["core-image-minimal".into()],
                    task: Some("testimage".into()),
                    force: false,
                },
                SystemTime::UNIX_EPOCH,
            )
            .expect("valid testimage request");
        assert!(matches!(
            actions.first(),
            Some(Action::QueueBackgroundJob(BackgroundJobSpec {
                kind: BackgroundJobKind::Test,
                context: BackgroundJobContext {
                    workspace: Some(Screen::Testing),
                    task: Some(task),
                    ..
                },
                ..
            })) if task == "testimage"
        ));
    }

    #[test]
    fn test_results_map_workspace_search_and_modal_keys_without_leakage() {
        assert_eq!(
            test_results_workspace_action(false, false, Input::Tab),
            Some(Action::CycleTestView)
        );
        assert_eq!(
            test_results_workspace_action(false, false, Input::Enter),
            Some(Action::DrillIntoSelectedTestResult)
        );
        assert_eq!(
            test_results_workspace_action(false, true, Input::Down),
            Some(Action::SelectTestCase { delta: 1 })
        );
        assert_eq!(
            test_results_workspace_action(false, true, Input::Esc),
            Some(Action::LeaveTestResultCases)
        );
        assert_eq!(
            test_results_workspace_action(false, false, Input::Char('I')),
            Some(Action::BeginTestResultImport)
        );
        assert_eq!(
            test_results_workspace_action(false, false, Input::Char('c')),
            Some(Action::BeginTestComparison)
        );
        assert_eq!(
            test_results_workspace_action(false, false, Input::Char('J')),
            Some(Action::BeginTestJunitExport)
        );
        assert_eq!(
            test_results_workspace_action(true, false, Input::Char('x')),
            Some(Action::AppendTestResultQuery('x')),
            "search input must not leak to the Testing workspace"
        );
        assert_eq!(
            test_result_import_dialog_action(Input::Char('/')),
            Some(Action::AppendTestResultImport('/'))
        );
        assert_eq!(
            test_result_import_dialog_action(Input::Enter),
            Some(Action::ConfirmTestResultImport)
        );
        assert_eq!(
            test_comparison_dialog_action(Input::Right),
            Some(Action::CycleTestComparisonField)
        );
        assert_eq!(
            test_comparison_dialog_action(Input::Char('p')),
            Some(Action::PreviewTestComparison)
        );
        assert_eq!(
            test_comparison_confirmation_action(Input::Enter),
            Some(Action::ConfirmTestComparison)
        );
        assert_eq!(
            test_junit_dialog_action(Input::Char('x')),
            Some(Action::AppendTestJunitDestination('x'))
        );
        assert_eq!(
            test_junit_dialog_action(Input::Enter),
            Some(Action::PreviewTestJunitExport)
        );
        assert_eq!(
            test_junit_confirmation_action(Input::Esc),
            Some(Action::CancelTestJunitExportPreview)
        );
        assert_eq!(
            test_comparison_workspace_action(Input::Char('l')),
            Some(Action::OpenSelectedTestTransitionLog)
        );
    }

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

    #[test]
    fn test_results_adapter_responses_map_to_identity_correlated_actions() {
        let baseline = yoctui_model::TestResultIdentity::new(
            "/results/baseline/testresults.json".into(),
            10,
            SystemTime::UNIX_EPOCH,
            "baseline".into(),
        )
        .unwrap();
        let candidate = yoctui_model::TestResultIdentity::new(
            "/results/candidate/testresults.json".into(),
            11,
            SystemTime::UNIX_EPOCH,
            "candidate".into(),
        )
        .unwrap();
        let import_request =
            yoctui_model::TestResultImportRequest::new(3, vec![baseline.path.clone()]).unwrap();
        assert_eq!(
            test_results_import_action(TestResultImportResponse {
                request: import_request.clone(),
                records: Vec::new(),
                limitations: vec!["empty fixture".into()],
            }),
            Action::TestResultsLoaded {
                request: import_request,
                records: Vec::new(),
                limitations: vec!["empty fixture".into()],
            }
        );

        let request =
            yoctui_model::TestComparisonRequest::new(4, baseline.clone(), candidate.clone())
                .unwrap();
        let comparison = TestComparison {
            baseline,
            candidate,
            transitions: Vec::new(),
        };
        assert_eq!(
            test_result_actions_for_runner_event(
                TestResultRunnerEvent::Completed {
                    operation: TestResultOperation::Comparison(request.clone()),
                    exit_code: Some(0),
                },
                Some(comparison.clone()),
                vec!["bounded import".into()],
            ),
            [Action::TestComparisonLoaded {
                request: request.clone(),
                comparison,
                limitations: vec!["bounded import".into()],
            }]
        );
        assert_eq!(
            test_result_actions_for_runner_event(
                TestResultRunnerEvent::Failed {
                    operation: TestResultOperation::Comparison(request.clone()),
                    exit_code: Some(6),
                },
                None,
                Vec::new(),
            ),
            [Action::TestComparisonFailed {
                request,
                message: "resulttool exited unsuccessfully with exit code 6".into(),
            }]
        );
    }

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

    #[test]
    fn security_workflow_maps_workspace_search_and_modal_keys_without_leakage() {
        assert_eq!(
            security_workspace_action(SecurityView::Cves, false, false, Input::Char('V')),
            Some(Action::Security(SecurityAction::BeginCveCheck))
        );
        assert_eq!(
            security_workspace_action(SecurityView::Sbom, false, false, Input::Down),
            Some(Action::Security(SecurityAction::SelectReport(1)))
        );
        assert_eq!(
            security_workspace_action(SecurityView::Sbom, true, false, Input::Down),
            Some(Action::Security(SecurityAction::SelectComponent(1)))
        );
        assert_eq!(
            security_workspace_action(SecurityView::Cves, false, true, Input::Char('x')),
            Some(Action::Security(SecurityAction::AppendQuery('x')))
        );
        assert_eq!(
            security_workspace_action(SecurityView::Cves, false, true, Input::Char('V')),
            Some(Action::Security(SecurityAction::AppendQuery('V'))),
            "search editing consumes workflow shortcuts"
        );

        let preview = yoctui_model::SecurityOperationPreview {
            id: yoctui_model::SecuritySessionId(7),
            scope: yoctui_model::SecurityScope::Image {
                target: "core-image-minimal".into(),
                machine: "qemux86-64".into(),
                distro: "poky".into(),
            },
            operation: yoctui_model::SecurityOperation::SbomBuild(BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: Some("create_recipe_sbom".into()),
                force: false,
            }),
            indexed_arguments: vec!["0: bitbake".into()],
            report_roots: vec!["/build/tmp/deploy/spdx".into()],
        };
        assert_eq!(
            security_dialog_action(&SecurityDialog::Operation(preview.clone()), Input::Enter),
            Some(Action::Security(SecurityAction::ConfirmOperation(preview)))
        );
        let mut import_editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
        import_editor.select_toml_value("root").unwrap();
        import_editor.editing = true;
        assert_eq!(
            security_dialog_action(
                &SecurityDialog::Import {
                    editor: import_editor,
                    validation_error: None,
                },
                Input::Char('V')
            ),
            Some(Action::EditActivePopup(PopupEditorCommand::Insert('V'))),
            "modal text editing does not leak CVE launch"
        );
        assert_eq!(
            security_dialog_action(
                &SecurityDialog::Cancellation(yoctui_model::SecuritySessionId(7)),
                Input::Esc
            ),
            Some(Action::Security(SecurityAction::CancelDialog))
        );
    }

    #[test]
    fn qa_workflow_maps_workspace_search_and_drill_keys_without_leakage() {
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('r')),
            Some(Action::Qa(QaAction::BeginSelectedCheck))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, false, Input::Down),
            Some(Action::Qa(QaAction::SelectCheck(1)))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, true, false, Input::Down),
            Some(Action::Qa(QaAction::SelectFinding(1)))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, true, Input::Char('r')),
            Some(Action::Qa(QaAction::AppendQuery('r'))),
            "search editing consumes QA run shortcuts"
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, true, false, Input::Esc),
            Some(Action::Qa(QaAction::LeaveDrill))
        );
        assert_eq!(
            qa_workspace_action(QaView::RecipeKernel, false, false, Input::Char('l')),
            Some(Action::Qa(QaAction::OpenSelectedSource))
        );
        assert_eq!(
            qa_workspace_action(QaView::LayerQa, false, false, Input::Tab),
            Some(Action::Qa(QaAction::CycleView))
        );
        assert_eq!(
            qa_workspace_action(QaView::LayerQa, false, false, Input::Down),
            Some(Action::Qa(QaAction::SelectLayer(1)))
        );
        assert_eq!(
            qa_workspace_action(QaView::LayerQa, false, false, Input::Char('r')),
            Some(Action::Qa(QaAction::BeginSelectedLayerCheck))
        );
        assert_eq!(
            qa_workspace_action(QaView::LayerQa, false, false, Input::Char('c')),
            Some(Action::Qa(QaAction::BeginLayerCancellation))
        );
        assert_eq!(
            qa_workspace_action(QaView::LayerQa, false, false, Input::Char('e')),
            Some(Action::Qa(QaAction::OpenSelectedLayerRoot))
        );
    }

    #[test]
    fn qa_workflow_maps_task_capability_response_without_reinterpreting_it() {
        let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
            name: "busybox".into(),
            file: "/layers/meta/recipes-core/busybox/busybox.bb".into(),
        })
        .unwrap();
        let snapshot = yoctui_model::QaCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            scope.clone(),
            vec![scope],
            vec![],
            vec!["one optional report root was unsafe".into()],
        )
        .unwrap();
        assert_eq!(
            qa_task_capability_action(QaTaskCapabilityResponse::Available(snapshot.clone())),
            Action::Qa(QaAction::CapabilityLoaded(snapshot.clone()))
        );
        assert_eq!(
            qa_task_capability_action(QaTaskCapabilityResponse::Partial(snapshot.clone())),
            Action::Qa(QaAction::CapabilityPartial {
                snapshot,
                limitations: vec!["one optional report root was unsafe".into()],
            })
        );
    }

    #[test]
    fn qa_workflow_maps_report_adapter_outcomes_without_parsing_them() {
        let request = yoctui_model::QaReportRequest::new(9, vec!["/build/reports".into()]).unwrap();
        assert_eq!(
            qa_report_response_action(QaReportResponse {
                request: request.clone(),
                outcome: QaReportScanOutcome::Empty,
            }),
            Action::Qa(QaAction::ReportsLoaded {
                request: request.clone(),
                reports: Vec::new(),
                limitations: Vec::new(),
            })
        );
        assert_eq!(
            qa_report_response_action(QaReportResponse {
                request: request.clone(),
                outcome: QaReportScanOutcome::Partial {
                    reports: Vec::new(),
                    limitations: vec!["one exact report was malformed".into()],
                },
            }),
            Action::Qa(QaAction::ReportsLoaded {
                request: request.clone(),
                reports: Vec::new(),
                limitations: vec!["one exact report was malformed".into()],
            })
        );
        assert_eq!(
            qa_report_error_action(request.clone(), QaReportAdapterError::Cancelled),
            Action::Qa(QaAction::ReportsCancelled(request.clone()))
        );
        assert_eq!(
            qa_report_error_action(request.clone(), QaReportAdapterError::Timeout(30)),
            Action::Qa(QaAction::ReportsTimedOut(request.clone()))
        );
        assert_eq!(
            qa_report_error_action(
                request.clone(),
                QaReportAdapterError::WorkerLost("channel closed".into())
            ),
            Action::Qa(QaAction::ReportsLost {
                request: request.clone(),
                message: "channel closed".into(),
            })
        );
        assert!(matches!(
            qa_report_error_action(
                request,
                QaReportAdapterError::PermissionDenied("/build/reports".into())
            ),
            Action::Qa(QaAction::ReportsFailed {
                kind: QaReportFailureKind::PermissionDenied,
                ..
            })
        ));
    }

    #[test]
    fn qa_workflow_maps_layer_capability_and_runner_events_mechanically() {
        let layer =
            yoctui_model::QaLayerIdentity::new("meta-demo".into(), "/layers/meta-demo".into())
                .unwrap();
        let configured = yoctui_model::QaConfiguredLayerCapability::new(
            yoctui_model::QaCheckId::new("layer-meta-demo".into()).unwrap(),
            layer.clone(),
            vec!["walnascar".into()],
            yoctui_model::QaLayerRunCapability::Disabled("tool unavailable".into()),
            vec!["tool unavailable".into()],
        )
        .unwrap();
        let snapshot = yoctui_model::QaLayerCapabilitySnapshot::new(
            Some("6.0".into()),
            "/build".into(),
            layer,
            vec![configured],
            vec!["tool unavailable".into()],
        )
        .unwrap();
        assert_eq!(
            qa_layer_capability_action(QaLayerCapabilityResponse::Partial(snapshot.clone())),
            Action::Qa(QaAction::LayerCapabilityPartial {
                snapshot,
                limitations: vec!["tool unavailable".into()],
            })
        );

        let timestamp = SystemTime::UNIX_EPOCH;
        let session = yoctui_model::QaLayerSessionId(7);
        assert_eq!(
            qa_layer_runner_action(QaLayerRunnerEvent::Started { id: session }, timestamp),
            Some(Action::Qa(QaAction::LayerSessionRunning(session)))
        );
        assert_eq!(
            qa_layer_runner_action(
                QaLayerRunnerEvent::Output {
                    id: session,
                    stream: yoctui_model::QaOutputStream::Stderr,
                    line: "warning".into(),
                    truncated: false,
                },
                timestamp,
            ),
            Some(Action::Qa(QaAction::LayerSessionOutput {
                session,
                stream: yoctui_model::QaOutputStream::Stderr,
                line: "warning".into(),
                truncated: false,
            }))
        );
        assert_eq!(
            qa_layer_runner_action(
                QaLayerRunnerEvent::TimedOut {
                    id: session,
                    forced: true,
                    exit_code: None,
                },
                timestamp,
            ),
            Some(Action::Qa(QaAction::TimeoutLayerSession {
                session,
                forced: true,
                exit_code: None,
                finished_at: timestamp,
            }))
        );
        assert_eq!(
            qa_layer_runner_action(
                QaLayerRunnerEvent::Lost {
                    id: session,
                    message: "channel lost".into(),
                },
                timestamp,
            ),
            Some(Action::Qa(QaAction::LoseLayerSession {
                session,
                message: "channel lost".into(),
                finished_at: timestamp,
            }))
        );
    }

    #[test]
    fn qa_workflow_dialogs_map_only_typed_confirmation_and_edit_actions() {
        let scope = yoctui_model::QaScope::new(yoctui_model::RecipeIdentity {
            name: "linux-yocto".into(),
            file: "/layers/meta/recipes-kernel/linux/linux-yocto.bb".into(),
        })
        .unwrap();
        let preview = yoctui_model::QaOperationPreview {
            id: yoctui_model::QaOperationId(7),
            check: yoctui_model::QaCheckId::new("kernel-config".into()).unwrap(),
            family: yoctui_model::QaCheckFamily::KernelConfiguration,
            scope,
            request: BuildRequest {
                targets: vec!["linux-yocto".into()],
                task: Some("kernel_configcheck".into()),
                force: false,
            },
            indexed_arguments: vec!["0: bitbake".into()],
            report_roots: vec!["/build/tmp/log/qa".into()],
            limitations: vec![],
        };
        assert_eq!(
            qa_dialog_action(&QaDialog::Operation(preview.clone()), Input::Enter),
            Some(Action::Qa(QaAction::ConfirmOperation(preview)))
        );
        assert_eq!(
            {
                let mut editor = yoctui_model::PopupEditor::new("root = \"/reports\"\n".into());
                editor.select_toml_value("root").unwrap();
                editor.editing = true;
                qa_dialog_action(
                    &QaDialog::Import {
                        editor,
                        validation_error: None,
                    },
                    Input::Char('r'),
                )
            },
            Some(Action::EditActivePopup(PopupEditorCommand::Insert('r'))),
            "modal text editing does not leak QA run"
        );
        assert_eq!(
            qa_dialog_action(
                &QaDialog::Cancellation {
                    session: yoctui_model::QaSessionId(3),
                    background_job: BackgroundJobId(9),
                },
                Input::Enter,
            ),
            Some(Action::Qa(QaAction::ConfirmCancellation(
                yoctui_model::QaSessionId(3)
            )))
        );
        let layer_preview = yoctui_model::QaLayerOperationPreview {
            id: yoctui_model::QaLayerOperationId(4),
            check: yoctui_model::QaCheckId::new("yocto-check-layer".into()).unwrap(),
            layer: yoctui_model::QaLayerIdentity::new("meta".into(), "/layers/meta".into())
                .unwrap(),
            executable: yoctui_model::QaExecutableIdentity::new(
                "/poky/scripts/yocto-check-layer".into(),
                10,
                SystemTime::UNIX_EPOCH,
            )
            .unwrap(),
            arguments: vec!["--layer".into(), "/layers/meta".into()],
            indexed_arguments: vec!["0: /poky/scripts/yocto-check-layer".into()],
            report_roots: vec![],
            limitations: vec![],
        };
        assert_eq!(
            qa_dialog_action(
                &QaDialog::LayerOperation(layer_preview.clone()),
                Input::Enter
            ),
            Some(Action::Qa(QaAction::ConfirmLayerOperation(layer_preview)))
        );
        assert_eq!(
            qa_dialog_action(
                &QaDialog::LayerCancellation(yoctui_model::QaLayerSessionId(4)),
                Input::Enter,
            ),
            Some(Action::Qa(QaAction::ConfirmLayerCancellation(
                yoctui_model::QaLayerSessionId(4)
            )))
        );
        assert_eq!(
            qa_dialog_action(
                &QaDialog::Import {
                    editor: yoctui_model::PopupEditor::new("root = \"\"\n".into()),
                    validation_error: None,
                },
                Input::Tab
            ),
            None
        );
    }

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
}
