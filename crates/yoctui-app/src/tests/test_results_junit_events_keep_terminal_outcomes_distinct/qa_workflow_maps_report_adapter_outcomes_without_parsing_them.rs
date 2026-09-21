use super::*;

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
