use super::*;

#[test]
fn qa_check_workflow_search_filter_selection_drill_and_exact_opens() {
    let mut state = QaState::default();
    load(&mut state);
    let request = QaReportRequest::new(1, vec!["/reports".into()]).unwrap();
    state.inventory = QaReportInventoryState::Loading {
        request: request.clone(),
    };
    let _ = update_qa(
        &mut state,
        QaAction::ReportsLoaded {
            request,
            reports: vec![report(vec![
                finding(QaFindingStatus::Failed, "failure"),
                finding(QaFindingStatus::Warning, "warning"),
            ])],
            limitations: vec![],
        },
    );
    let _ = update_qa(&mut state, QaAction::Drill);
    assert!(state.drilled);
    let _ = update_qa(&mut state, QaAction::CycleStatusFilter);
    assert_eq!(state.status_filter, QaStatusFilter::Failed);
    assert_eq!(state.visible_findings().len(), 1);
    let _ = update_qa(&mut state, QaAction::BeginSearch);
    for character in "devmem".chars() {
        let _ = update_qa(&mut state, QaAction::AppendQuery(character));
    }
    assert_eq!(state.visible_findings().len(), 1);
    let source = state.selected_finding().unwrap().source.clone().unwrap();
    assert_eq!(
        update_qa(&mut state, QaAction::OpenSelectedSource).effect,
        Some(QaEffect::OpenSource(source))
    );
    assert!(matches!(
        update_qa(&mut state, QaAction::OpenSelectedReport).effect,
        Some(QaEffect::OpenReport(_))
    ));
    assert_eq!(
        update_qa(&mut state, QaAction::OpenProvider).effect,
        Some(QaEffect::OpenProvider(scope("linux-yocto").recipe))
    );
}
