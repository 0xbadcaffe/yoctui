//! Regression tests grouped around test_workflow_model_attaches_managed_builds_and_rejects_stale_results.
use super::*;

#[test]
fn test_workflow_model_attaches_managed_builds_and_rejects_stale_results() {
    let mut app = test_workflow_app();
    let _ = update(&mut app, Action::SelectTestFamily { delta: 2 });
    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    let _ = update(&mut app, Action::PreviewTestLaunch);
    let Some(Effect::StartTestBuildSession {
        id,
        family: _,
        request,
    }) = update(&mut app, Action::ConfirmTestLaunch)
    else {
        panic!("test build effect");
    };
    assert_eq!(request.task.as_deref(), Some("testimage"));
    assert!(app.test_session(id).unwrap().background_job_id.is_none());
    let job_id = BackgroundJobId(44);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id: job_id,
            kind: BackgroundJobKind::Test,
            title: "Image runtime test".into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Testing),
                image: Some("core-image-minimal".into()),
                task: Some("testimage".into()),
                ..BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::AttachTestBuildSession {
            id,
            background_job_id: job_id,
        },
    );
    assert_eq!(
        app.test_session(id).unwrap().background_job_id,
        Some(job_id)
    );
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::CompleteTestSession {
            id,
            exit_code: 0,
            result_paths: vec!["relative/testresults.json".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let _ = update(
        &mut app,
        Action::TestSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::TestSessionRunning { id });
    let _ = update(
        &mut app,
        Action::CompleteTestSession {
            id,
            exit_code: 0,
            result_paths: vec!["/build/testresults.json".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().status,
        BackgroundJobStatus::Succeeded
    );
    assert_eq!(
        app.test_session(id).unwrap().result_paths,
        [PathBuf::from("/build/testresults.json")]
    );
}

#[test]
fn test_workflow_model_records_failure_loss_and_stale_terminal_events() {
    fn queue_selftest(app: &mut App) -> TestSessionId {
        let request = TestSelftestRequest::new(
            "/workspace/oe-selftest".into(),
            TestFamily::OeSelftest,
            None,
            1,
            false,
            false,
        )
        .unwrap();
        let Some(Effect::StartTestSession { id, .. }) =
            queue_test_session(app, TestOperation::Selftest(request))
        else {
            panic!("selftest session");
        };
        id
    }

    let mut failed = test_workflow_app();
    let failed_id = queue_selftest(&mut failed);
    let failed_job = failed
        .test_session(failed_id)
        .unwrap()
        .background_job_id
        .unwrap();
    let _ = update(
        &mut failed,
        Action::FailTestSession {
            id: failed_id,
            message: "runner timed out after forced termination".into(),
            exit_code: None,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = failed.background_jobs.get(failed_job).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Failed);
    assert_eq!(
        job.error.as_ref().and_then(|error| error.detail.as_deref()),
        Some("runner timed out after forced termination")
    );

    let mut lost = test_workflow_app();
    let lost_id = queue_selftest(&mut lost);
    let lost_job = lost
        .test_session(lost_id)
        .unwrap()
        .background_job_id
        .unwrap();
    let _ = update(
        &mut lost,
        Action::LoseTestSession {
            id: lost_id,
            message: "runner event channel closed".into(),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        lost.background_jobs.get(lost_job).unwrap().status,
        BackgroundJobStatus::Lost
    );

    let mut timed_out = test_workflow_app();
    let timed_out_id = queue_selftest(&mut timed_out);
    let _ = update(
        &mut timed_out,
        Action::TimeoutTestSession {
            id: timed_out_id,
            forced: true,
            exit_code: None,
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        timed_out.test_session(timed_out_id).unwrap().outcome,
        Some(TestSessionOutcome::TimedOut)
    );

    let ignored = lost.background_jobs.ignored_transitions;
    let _ = update(
        &mut lost,
        Action::FailTestSession {
            id: TestSessionId(u64::MAX),
            message: "stale".into(),
            exit_code: Some(1),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(lost.background_jobs.ignored_transitions, ignored + 1);
}

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

#[test]
fn test_results_popup_editors_share_selection_navigation_and_clipboard() {
    let mut app = test_workflow_app();
    let _ = update(&mut app, Action::BeginTestResultImport);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestResultImportTomlEditor { editor, .. })
            if editor.selected_text() == Some("")
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value.is_empty()
    ));

    app.dialogs.clear();
    let baseline = test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
    let candidate = test_results_record(
        "candidate",
        "candidate",
        &[("case", TestCaseOutcome::Failed)],
    );
    load_test_results(&mut app, vec![baseline, candidate], Vec::new());
    let _ = update(&mut app, Action::BeginTestComparison);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestComparisonTomlEditor { editor, .. })
            if editor.selected_text().is_some_and(|value| value.starts_with("/build/results/"))
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value.starts_with("/build/results/")
    ));
}

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

#[test]
fn build_environment_requires_a_verified_correlated_connection() {
    let profile = BuildEnvironmentProfile {
        source_dir: PathBuf::from("/workspace/poky"),
        build_dir: PathBuf::from("/workspace/build"),
        init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
    };
    let mut app = App::new_unconfigured(16, 4096);
    assert_eq!(app.screen, Screen::BuildEnvironment);
    assert_eq!(app.focus, FocusTarget::Navigator);
    let request = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    assert_eq!(update(&mut app, Action::Start(request.clone())), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("Configure and verify a BitBake environment first")
    );

    assert_eq!(
        update(&mut app, Action::ConfigureBuildEnvironment(profile.clone())),
        None
    );
    let Some(Effect::VerifyBuildEnvironment { generation, .. }) =
        update(&mut app, Action::BeginBuildEnvironmentVerification)
    else {
        panic!("verification effect");
    };
    let _ = update(
        &mut app,
        Action::BuildEnvironmentVerified {
            generation: generation + 1,
        },
    );
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Verifying { .. }
    ));
    let _ = update(&mut app, Action::BuildEnvironmentVerified { generation });
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Connected(profile)
    );
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
}

#[test]
fn build_environment_form_edits_typed_profile_and_clears_inventory() {
    let mut app = App::new_unconfigured(8, 512);
    app.available_images = vec!["core-image-minimal".into()];
    let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
    let _ = update(&mut app, Action::AppendBuildEnvironmentField('/'));
    for c in "src".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
    for c in "/build".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
    for c in "/env".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::ApplyBuildEnvironmentProfile);
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Configured(_)
    ));
    assert!(app.available_images.is_empty());
}

#[test]
fn recipe_refresh_rebuilds_authoritative_image_inventory() {
    let mut app = App::new(16, 4096);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "base-files".into(),
                ..Recipe::default()
            },
            Recipe {
                name: "core-image-minimal".into(),
                ..Recipe::default()
            },
        ]),
    );
    assert_eq!(app.available_images, vec!["core-image-minimal"]);
}

#[test]
fn build_environment_toml_editor_applies_profile_from_popup() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentEditor(editor)) if !editor.editing
    ));
    if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() {
        editor.editing = true;
        editor.text = "source = \"/src/poky\"\nbuild = \"/src/build\"\nscript = \"/src/poky/oe-init-build-env\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::ApplyBuildEnvironmentEditor);
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Configured(_)
    ));
}

#[test]
fn build_environment_popup_uses_shared_selection_navigation_and_clipboard() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(
        &mut app,
        Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
            source_dir: "/old/source".into(),
            build_dir: "/old/build".into(),
            init_script: "/old/source/oe-init-build-env".into(),
        }),
    );
    let _ = update(&mut app, Action::OpenBuildEnvironmentEditor);
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value == "/old/source"
    ));
    let _ = update(
        &mut app,
        Action::EditActivePopup(PopupEditorCommand::ToggleInsert),
    );
    for character in "/new/source".chars() {
        let _ = update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Insert(character)),
        );
    }
    let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() else {
        panic!("build environment editor");
    };
    assert!(editor.text.starts_with("source = \"/new/source\""));
}

#[test]
fn build_environment_rejects_relative_profiles_and_preserves_unconfigured_state() {
    let mut app = App::new_unconfigured(16, 4096);
    let _ = update(
        &mut app,
        Action::ConfigureBuildEnvironment(BuildEnvironmentProfile {
            source_dir: PathBuf::from("poky"),
            build_dir: PathBuf::from("/workspace/build"),
            init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
        }),
    );
    assert_eq!(app.build_environment, BuildEnvironmentState::Unconfigured);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("absolute"))
    );
}

#[test]
fn build_environment_clone_request_rejects_unsafe_revision_and_destination() {
    let mut request = BuildEnvironmentCloneRequest {
        repository: "https://example.invalid/poky".into(),
        destination: PathBuf::from("/workspace/poky"),
        revision: Some("main;rm -rf".into()),
    };
    assert!(request.validate().is_err());
    request.revision = Some("scarthgap".into());
    request.destination = PathBuf::from("relative/poky");
    assert!(request.validate().is_err());
}

#[test]
fn build_environment_clone_draft_does_not_guess_machine_paths() {
    let mut app = App::new_unconfigured(8, 512);
    assert!(update(&mut app, Action::OpenBuildEnvironmentCloneEditor).is_none());
    let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog() else {
        panic!("clone editor was not opened");
    };
    let fields: toml::Table = toml::from_str(&editor.text).unwrap();
    for key in ["repository", "destination", "revision", "build"] {
        assert_eq!(fields[key].as_str(), Some(""));
    }
    assert!(update(&mut app, Action::ReviewBuildEnvironmentClone).is_none());
    assert!(!matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentCloneReview(_))
    ));
}

#[test]
fn build_environment_clone_editor_requires_review_before_emitting_clone_effect() {
    let mut app = App::new_unconfigured(8, 512);
    let _ = update(&mut app, Action::OpenBuildEnvironmentCloneEditor);
    if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut() {
        editor.text = "repository = \"https://git.yoctoproject.org/poky\"\ndestination = \"/tmp/poky\"\nrevision = \"\"\nbuild = \"/tmp/poky/build\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::ReviewBuildEnvironmentClone);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildEnvironmentCloneReview(_))
    ));
    assert!(matches!(
        update(&mut app, Action::ConfirmBuildEnvironmentClone),
        Some(Effect::CloneBuildEnvironment(_))
    ));
}

#[test]
fn popup_editor_replaces_selection_and_moves_to_line_bounds() {
    let mut editor = PopupEditor::new("path = \"old\"\nnext = \"value\"".into());
    editor.select_range(8, 11);
    assert_eq!(editor.selected_text(), Some("old"));
    let replacement = "/test/path/poky";
    editor.insert(replacement);
    assert!(editor.text.contains(replacement));
    editor.end();
    assert_eq!(editor.cursor, editor.text.find('\n').unwrap());
    editor.home();
    assert_eq!(editor.cursor, 0);
}

#[test]
fn popup_editor_selects_a_toml_value_and_undoes_replacement() {
    let mut editor = PopupEditor::new("path = \"/old\"\nmode = \"safe\"\n".into());
    editor.select_toml_value("path").unwrap();
    assert_eq!(editor.selected_text(), Some("/old"));
    editor.insert("/new");
    assert_eq!(editor.text, "path = \"/new\"\nmode = \"safe\"\n");
    assert!(editor.undo());
    assert_eq!(editor.text, "path = \"/old\"\nmode = \"safe\"\n");
    assert!(!editor.undo());
}

#[test]
fn popup_editor_selects_native_toml_boolean_and_integer_values() {
    let mut editor = PopupEditor::new("enabled = true\njobs = 12 # bounded\n".into());
    editor.select_toml_value("enabled").unwrap();
    assert_eq!(editor.selected_text(), Some("true"));
    editor.cursor = editor.text.find("jobs").unwrap();
    editor.select_toml_value_at_cursor().unwrap();
    assert_eq!(editor.selected_text(), Some("12"));
}

#[test]
fn popup_editor_supports_unicode_navigation_copy_paste_and_backspace() {
    let mut editor = PopupEditor::new("path = \"hé\"\n".into());
    editor.select_toml_value("path").unwrap();
    assert_eq!(editor.copy_selection_or_line(), "hé");
    editor.editing = true;
    editor.insert("x");
    editor.paste();
    assert_eq!(editor.text, "path = \"xhé\"\n");
    editor.left();
    editor.backspace();
    assert_eq!(editor.text, "path = \"xé\"\n");
    editor.home();
    assert_eq!(editor.cursor, 0);
    editor.end();
    assert_eq!(editor.cursor, "path = \"xé\"".len());
}

#[test]
fn junit_popup_editor_routes_selection_navigation_and_clipboard_actions() {
    let result = test_results_record("candidate", "candidate", &[]);
    let mut app = test_workflow_app();
    app.result_tool_capability = ResultToolCapability::Available("/workspace/resulttool".into());
    load_test_results(&mut app, vec![result], Vec::new());
    let _ = update(&mut app, Action::BeginTestJunitExport);
    let _ = update(&mut app, Action::SelectTestJunitDestination);
    let _ = update(&mut app, Action::AppendTestJunitTomlEditor('x'));
    assert!(matches!(
        update(&mut app, Action::CopyTestJunitTomlEditor),
        Some(Effect::CopyToClipboard(value)) if value.contains("destination")
    ));
    let _ = update(&mut app, Action::MoveTestJunitTomlEditorHome);
    let _ = update(&mut app, Action::MoveTestJunitTomlEditorEnd);
    let _ = update(&mut app, Action::PasteTestJunitTomlEditor);
    let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog() else {
        panic!("JUnit editor");
    };
    assert!(
        editor
            .text
            .contains("destination = \"x\"destination = \"x\"")
    );
}

#[test]
fn raw_navigation_is_unique_grouped_and_palette_reachable() {
    let raw_destinations = NAVIGATOR_SCREENS
        .iter()
        .enumerate()
        .filter_map(|(index, screen)| (*screen == Screen::RawMode).then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(raw_destinations, [17]);
    assert_eq!(
        NAVIGATOR_COMPATIBILITY_DESTINATIONS[17],
        WorkspaceDestination::RawMode
    );
    assert_eq!(NAVIGATOR_GROUPS[4].label, "TOOLS");
    assert!((NAVIGATOR_GROUPS[4].start..NAVIGATOR_GROUPS[4].end).contains(&17));

    let mut app = App::new(16, 4096);
    let raw_commands = app
        .command_palette_commands()
        .into_iter()
        .filter(|command| command.id == CommandId::OpenRawMode)
        .collect::<Vec<_>>();
    assert_eq!(raw_commands.len(), 1);
    assert!(raw_commands[0].enabled());
    assert_eq!(
        command_action(&app, CommandId::OpenRawMode),
        Action::Open(Screen::RawMode)
    );

    assert_eq!(update(&mut app, Action::Open(Screen::RawMode)), None);
    assert_eq!(app.navigator_selection, 17);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(app.inspector_mode(), InspectorMode::Navigator);
    assert_eq!(
        update(&mut app, Action::CycleFocus { backwards: false }),
        None
    );
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.inspector_mode(), InspectorMode::RawCommand);
}
