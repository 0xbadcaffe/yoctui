use super::*;

#[test]
fn test_results_reducer_previews_exact_comparison_and_rejects_inconsistent_results() {
    let baseline = test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
    let candidate = test_results_record(
        "candidate",
        "candidate",
        &[("case", TestCaseOutcome::Failed)],
    );
    let mut app = test_workflow_app();
    app.result_tool_capability = ResultToolCapability::Available("/workspace/resulttool".into());
    load_test_results(
        &mut app,
        vec![baseline.clone(), candidate.clone()],
        Vec::new(),
    );
    let _ = update(&mut app, Action::BeginTestComparison);
    let _ = update(&mut app, Action::PreviewTestComparison);
    let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog().cloned() else {
        panic!("comparison preview");
    };
    assert_eq!(
        preview.argv,
        [
            PathBuf::from("/workspace/resulttool"),
            "regression-file".into(),
            baseline.identity.path.clone(),
            candidate.identity.path.clone(),
        ]
    );
    let Some(Effect::CompareTestResults(request)) = update(&mut app, Action::ConfirmTestComparison)
    else {
        panic!("comparison effect");
    };
    let comparison = TestComparison::between(&baseline, &candidate).unwrap();
    let _ = update(
        &mut app,
        Action::TestComparisonLoaded {
            request: request.clone(),
            comparison,
            limitations: vec!["resulttool omitted optional metadata".into()],
        },
    );
    assert!(matches!(
        app.test_comparison,
        TestComparisonState::Partial { .. }
    ));
    assert_eq!(
        app.selected_test_transition().unwrap().category,
        TestComparisonCategory::Regression
    );
    assert!(matches!(
        update(&mut app, Action::OpenSelectedTestTransitionLog),
        Some(Effect::OpenInEditor(path))
            if path == Path::new("/build/logs/candidate-case.log")
    ));

    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::TestComparisonFailed {
            request,
            message: "late failure".into(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}
