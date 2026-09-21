use super::*;

#[test]
fn test_workflow_model_navigates_previews_and_runs_bounded_selftests() {
    let mut app = test_workflow_app();
    let testing_index = NAVIGATOR_SCREENS
        .iter()
        .position(|screen| *screen == Screen::Testing)
        .unwrap();
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = testing_index;
    app.test_capability = TestCapability::default();
    assert_eq!(
        update(&mut app, Action::ActivateNavigator),
        Some(Effect::InspectTestCapability)
    );
    assert_eq!(app.screen, Screen::Testing);
    let capability = test_workflow_app().test_capability;
    let _ = update(&mut app, Action::TestCapabilityLoaded(capability));
    let _ = update(&mut app, Action::BeginSelectedTestLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchTomlEditor { editor, .. })
            if editor.selected_text() == Some("all")
    ));
    if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() {
        editor.text = "family = \"OE selftest\"\nmachine = \"qemux86-64\"\ndistro = \"poky\"\nimage = \"core-image-minimal\"\nscope = \"selected\"\nselector = \"tinfoil.TinfoilTests.test_getvar\"\nparallelism = 8\nverbose = false\nskip_network = false\n".into();
    }
    let _ = update(&mut app, Action::PreviewTestLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestLaunchConfirmation(
            TestLaunchPreview::Selftest(request)
        )) if request.parallelism == 8
            && request.selector.as_deref() == Some("tinfoil.TinfoilTests.test_getvar")
    ));
    let Some(Effect::StartTestSession { id, operation }) =
        update(&mut app, Action::ConfirmTestLaunch)
    else {
        panic!("selftest effect");
    };
    assert!(matches!(
        operation,
        TestOperation::Selftest(TestSelftestRequest {
            family: TestFamily::OeSelftest,
            ..
        })
    ));
    let job_id = app.test_session(id).unwrap().background_job_id.unwrap();
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().kind,
        BackgroundJobKind::Test
    );
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
        Action::AppendTestSessionOutput {
            id,
            stream: TestOutputStream::Stderr,
            line: "one warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestCancellationConfirmation(candidate)) if *candidate == id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmTestSessionCancellation),
        Some(Effect::CancelTestSession(id))
    );
    let _ = update(
        &mut app,
        Action::RejectTestSessionCancellation {
            id,
            message: "still stopping".into(),
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().status,
        BackgroundJobStatus::Running
    );
    let _ = update(&mut app, Action::BeginActiveTestSessionCancellation);
    let _ = update(&mut app, Action::ConfirmTestSessionCancellation);
    let _ = update(
        &mut app,
        Action::CancelTestSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(job_id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Cancelled);
    assert!(job.output[0].truncated);
}
