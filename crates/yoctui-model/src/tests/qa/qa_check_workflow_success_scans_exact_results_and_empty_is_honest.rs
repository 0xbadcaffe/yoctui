use super::*;

#[test]
fn qa_check_workflow_success_scans_exact_results_and_empty_is_honest() {
    let mut state = QaState::default();
    load(&mut state);
    let id = start(&mut state);
    let transition = update_qa(
        &mut state,
        QaAction::CompleteSession {
            session: id,
            result_paths: vec!["/build/tmp/log/qa/report.json".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let Some(QaEffect::ImportReports(request)) = transition.effect else {
        panic!("expected report scan")
    };
    assert_eq!(
        request.paths,
        [PathBuf::from("/build/tmp/log/qa/report.json")]
    );
    assert!(matches!(
        state.inventory,
        QaReportInventoryState::Loading { .. }
    ));

    let mut other = QaState::default();
    load(&mut other);
    let id = start(&mut other);
    let transition = update_qa(
        &mut other,
        QaAction::CompleteSession {
            session: id,
            result_paths: vec![],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert!(transition.effect.is_none());
    assert_eq!(
        other.sessions[0].message.as_deref(),
        Some("no report supplied")
    );
}
