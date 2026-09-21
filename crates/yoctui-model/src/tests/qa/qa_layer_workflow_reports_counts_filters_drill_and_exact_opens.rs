use super::*;

#[test]
fn qa_layer_workflow_reports_counts_filters_drill_and_exact_opens() {
    let mut state = QaState::default();
    load_layer(&mut state);
    let id = start_layer(&mut state);
    let transition = update_qa(
        &mut state,
        QaAction::CompleteLayerSession {
            session: id,
            exit_code: 0,
            result_paths: vec!["/build/tmp/log/qa-layer/report.json".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let Some(QaEffect::ImportReports(request)) = transition.effect else {
        panic!("expected exact layer report scan")
    };
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLoaded {
            request,
            reports: vec![layer_report(vec![
                layer_finding(QaFindingStatus::Failed, "layer-fail"),
                layer_finding(QaFindingStatus::Warning, "layer-warning"),
            ])],
            limitations: vec![],
        },
    );
    assert_eq!(
        state.layer_finding_counts(&layer("meta")),
        QaFindingCounts {
            failed: 1,
            warnings: 1,
            ..QaFindingCounts::default()
        }
    );
    let _ = update_qa(&mut state, QaAction::CycleStatusFilter);
    assert_eq!(state.status_filter, QaStatusFilter::Failed);
    assert_eq!(state.visible_findings().len(), 1);
    let _ = update_qa(&mut state, QaAction::BeginSearch);
    for character in "compatibility".chars() {
        let _ = update_qa(&mut state, QaAction::AppendQuery(character));
    }
    assert_eq!(state.visible_layers().len(), 1);
    let _ = update_qa(&mut state, QaAction::Drill);
    assert!(state.drilled);
    assert!(matches!(
        update_qa(&mut state, QaAction::OpenSelectedSource).effect,
        Some(QaEffect::OpenSource(_))
    ));
    assert_eq!(
        update_qa(&mut state, QaAction::OpenSelectedLayerRoot).effect,
        Some(QaEffect::OpenLayerRoot(layer("meta")))
    );
}
