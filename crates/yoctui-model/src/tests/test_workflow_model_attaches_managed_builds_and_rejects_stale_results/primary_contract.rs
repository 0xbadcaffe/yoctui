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
