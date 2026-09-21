use super::*;

#[test]
fn qa_check_workflow_report_generations_bounds_partial_and_terminal_states() {
    let mut state = QaState::default();
    load(&mut state);
    let transition = update_qa(
        &mut state,
        QaAction::ConfirmImport("root = \"/reports\"\n".into()),
    );
    let Some(QaEffect::ImportReports(request)) = transition.effect else {
        panic!("expected report import")
    };
    let stale = QaReportRequest::new(request.generation + 1, request.paths.clone()).unwrap();
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLoaded {
            request: stale,
            reports: vec![report(vec![finding(QaFindingStatus::Failed, "a")])],
            limitations: vec![],
        },
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Loading { .. }
    ));
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLoaded {
            request: request.clone(),
            reports: vec![report(vec![
                finding(QaFindingStatus::Failed, "a"),
                finding(QaFindingStatus::Warning, "b"),
            ])],
            limitations: vec!["one input record was malformed".into()],
        },
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Partial { .. }
    ));
    assert_eq!(state.visible_findings().len(), 2);

    let refresh = update_qa(&mut state, QaAction::RefreshReports);
    let Some(QaEffect::ImportReports(refresh_request)) = refresh.effect else {
        panic!("expected refresh")
    };
    let _ = update_qa(
        &mut state,
        QaAction::ReportsFailed {
            request: refresh_request,
            kind: QaReportFailureKind::PermissionDenied,
            message: "permission denied".into(),
        },
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Failed {
            kind: QaReportFailureKind::PermissionDenied,
            ..
        }
    ));

    let mut findings = (0..=MAX_QA_FINDINGS)
        .map(|index| finding(QaFindingStatus::Warning, &format!("finding{index}")))
        .collect::<Vec<_>>();
    findings.push(findings[0].clone());
    let (reports, limitations) = normalize_qa_reports(
        vec![report(findings)],
        &[QaCheckId::new("kernel-config".into()).unwrap()],
        &[QaFindingScope::Recipe(scope("linux-yocto"))],
    );
    assert_eq!(reports[0].findings.len(), MAX_QA_FINDINGS);
    assert!(
        limitations
            .iter()
            .any(|value| value.contains("beyond the model bound"))
    );

    let cancelled = QaReportRequest::new(request.generation + 10, vec!["/reports".into()]).unwrap();
    state.inventory = QaReportInventoryState::Loading {
        request: cancelled.clone(),
    };
    let _ = update_qa(&mut state, QaAction::ReportsCancelled(cancelled.clone()));
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Cancelled { .. }
    ));
    state.inventory = QaReportInventoryState::Loading {
        request: cancelled.clone(),
    };
    let _ = update_qa(&mut state, QaAction::ReportsTimedOut(cancelled.clone()));
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::TimedOut { .. }
    ));
    state.inventory = QaReportInventoryState::Loading {
        request: cancelled.clone(),
    };
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLost {
            request: cancelled,
            message: "worker channel closed".into(),
        },
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Lost { .. }
    ));

    let empty = QaReportRequest::new(request.generation + 11, vec!["/reports".into()]).unwrap();
    state.inventory = QaReportInventoryState::Loading {
        request: empty.clone(),
    };
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLoaded {
            request: empty,
            reports: vec![],
            limitations: vec![],
        },
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::AvailableEmpty { .. }
    ));
}
