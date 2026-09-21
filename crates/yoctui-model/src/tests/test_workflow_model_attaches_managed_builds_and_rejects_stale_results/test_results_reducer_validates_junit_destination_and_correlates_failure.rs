use super::*;

#[test]
fn test_results_reducer_validates_junit_destination_and_correlates_failure() {
    let result = test_results_record(
        "candidate",
        "candidate",
        &[("case", TestCaseOutcome::Failed)],
    );
    let mut app = test_workflow_app();
    app.result_tool_capability = ResultToolCapability::Available("/workspace/resulttool".into());
    load_test_results(&mut app, vec![result.clone()], Vec::new());
    let _ = update(&mut app, Action::BeginTestJunitExport);
    if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "destination = \"/exports/candidate.xml\"\n".into();
        editor.cursor = editor.text.len();
    }
    let Some(Effect::InspectTestJunitDestination {
        result: inspected_result,
        destination,
    }) = update(&mut app, Action::PreviewTestJunitExport)
    else {
        panic!("destination inspection effect");
    };
    assert_eq!(inspected_result, result.identity);
    assert_eq!(destination, PathBuf::from("/exports/candidate.xml"));
    let _ = update(
        &mut app,
        Action::TestJunitDestinationInspected {
            result: result.identity.clone(),
            inspection: TestJunitDestinationInspection {
                requested: destination.clone(),
                canonical_parent: Some("/exports".into()),
                parent_exists: true,
                parent_is_directory: true,
                destination_exists: true,
                destination_is_symlink: false,
            },
        },
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestJunitTomlEditor {
            validation_error: Some(_),
            ..
        })
    ));
    assert!(
        update(&mut app, Action::PreviewTestJunitExport).is_some(),
        "the corrected retry requests fresh filesystem validation"
    );
    let _ = update(
        &mut app,
        Action::TestJunitDestinationInspected {
            result: result.identity.clone(),
            inspection: TestJunitDestinationInspection {
                requested: destination,
                canonical_parent: Some("/exports".into()),
                parent_exists: true,
                parent_is_directory: true,
                destination_exists: false,
                destination_is_symlink: false,
            },
        },
    );
    let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog().cloned() else {
        panic!("JUnit confirmation");
    };
    assert_eq!(
        preview.argv,
        [
            PathBuf::from("/workspace/resulttool"),
            "junit".into(),
            result.identity.path,
            "-j".into(),
            "/exports/candidate.xml".into(),
        ]
    );
    let Some(Effect::ExportTestJunit(request)) = update(&mut app, Action::ConfirmTestJunitExport)
    else {
        panic!("JUnit export effect");
    };
    let stale = TestJunitExportRequest {
        generation: request.generation + 1,
        ..request.clone()
    };
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::TestJunitExportSucceeded { request: stale },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let _ = update(
        &mut app,
        Action::TestJunitExportFailed {
            request: request.clone(),
            message: "resulttool exited 1".into(),
        },
    );
    assert!(matches!(
        app.test_junit_export,
        TestJunitExportState::Failed { .. }
    ));
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(&mut app, Action::TestJunitExportSucceeded { request });
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}
