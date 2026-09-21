use super::*;

#[test]
fn qemu_model_requires_cancellation_confirmation_and_rejects_stale_events() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    let Some(Effect::StartQemuSession { id, .. }) = update(&mut app, Action::ConfirmQemuLaunch)
    else {
        panic!("expected start");
    };
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuCancellationConfirmation(candidate)) if *candidate == id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmQemuSessionCancellation),
        Some(Effect::CancelQemuSession(id))
    );
    let job_id = app.qemu_session(id).expect("session").background_job_id;
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Cancelling)
    );
    let _ = update(
        &mut app,
        Action::RejectQemuSessionCancellation {
            id,
            message: "signal failed".into(),
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Running)
    );
    let _ = update(&mut app, Action::BeginQemuSessionCancellation { id });
    let _ = update(&mut app, Action::ConfirmQemuSessionCancellation);
    let _ = update(
        &mut app,
        Action::CancelQemuSession {
            id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Cancelled)
    );
    assert_eq!(
        app.qemu_session(id).and_then(|session| session.exit_code),
        Some(130)
    );

    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::QemuSessionRunning {
            id: QemuSessionId(99_999),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
}
