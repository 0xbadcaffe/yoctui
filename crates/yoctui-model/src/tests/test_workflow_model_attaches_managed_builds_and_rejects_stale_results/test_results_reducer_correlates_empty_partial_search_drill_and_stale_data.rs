use super::*;

#[test]
fn test_results_reducer_correlates_empty_partial_search_drill_and_stale_data() {
    let mut empty = test_workflow_app();
    let request = load_test_results(&mut empty, Vec::new(), Vec::new());
    assert!(matches!(
        empty.test_results,
        TestResultInventoryState::AvailableEmpty { .. }
    ));
    let ignored = empty.background_jobs.ignored_transitions;
    let _ = update(
        &mut empty,
        Action::TestResultsLost {
            request,
            message: "late worker loss".into(),
        },
    );
    assert_eq!(empty.background_jobs.ignored_transitions, ignored + 1);

    let baseline = test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
    let candidate = test_results_record(
        "candidate",
        "candidate",
        &[("case", TestCaseOutcome::Failed)],
    );
    let mut app = test_workflow_app();
    let old_request = load_test_results(
        &mut app,
        vec![candidate.clone(), baseline.clone(), candidate],
        vec!["one malformed adapter record was skipped".into()],
    );
    let TestResultInventoryState::Partial {
        records,
        limitations,
        ..
    } = &app.test_results
    else {
        panic!("partial inventory");
    };
    assert_eq!(records.len(), 2);
    assert!(limitations.iter().any(|value| value.contains("duplicate")));
    assert_eq!(app.test_result_selection.as_ref(), Some(&baseline.identity));
    let _ = update(&mut app, Action::BeginTestResultSearch);
    for character in "candidate".chars() {
        let _ = update(&mut app, Action::AppendTestResultQuery(character));
    }
    assert_eq!(
        app.test_result_selection.as_ref(),
        Some(
            &test_results_record(
                "candidate",
                "candidate",
                &[("case", TestCaseOutcome::Failed)]
            )
            .identity
        ),
        "search falls back to the first visible exact result"
    );
    assert_eq!(app.filtered_test_results().len(), 1);
    let _ = update(&mut app, Action::SelectTestResult { delta: 1 });
    assert_eq!(
        app.test_result_selection.as_ref(),
        Some(
            &test_results_record(
                "candidate",
                "candidate",
                &[("case", TestCaseOutcome::Failed)]
            )
            .identity
        )
    );
    let _ = update(&mut app, Action::DrillIntoSelectedTestResult);
    assert!(app.test_result_drilled);
    assert!(matches!(
        update(&mut app, Action::OpenSelectedTestCaseLog),
        Some(Effect::OpenInEditor(path))
            if path == Path::new("/build/logs/candidate-case.log")
    ));
    assert!(matches!(
        update(&mut app, Action::OpenSelectedTestResult),
        Some(Effect::OpenInEditor(path))
            if path == Path::new("/build/results/candidate/testresults.json")
    ));

    let Some(Effect::ImportTestResults(new_request)) = update(&mut app, Action::RefreshTestResults)
    else {
        panic!("refresh effect");
    };
    assert_ne!(old_request, new_request);
    let mut malformed = baseline;
    malformed.identity.path = "relative.json".into();
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::TestResultsLoaded {
            request: new_request,
            records: vec![malformed],
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    assert!(matches!(
        app.test_results,
        TestResultInventoryState::Loading { .. }
    ));
}
