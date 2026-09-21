use super::*;

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
