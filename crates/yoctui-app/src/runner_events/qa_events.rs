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
