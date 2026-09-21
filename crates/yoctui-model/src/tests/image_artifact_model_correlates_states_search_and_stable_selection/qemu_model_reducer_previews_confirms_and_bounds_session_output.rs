use super::*;

#[test]
fn qemu_model_reducer_previews_confirms_and_bounds_session_output() {
    let mut app = qemu_model_app();
    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert!(matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))));
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::PreviewQemuLaunch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QemuLaunchConfirmation(_))
    ));
    let effect = update(&mut app, Action::ConfirmQemuLaunch);
    let Some(Effect::StartQemuSession { id, request }) = effect else {
        panic!("expected typed runqemu start effect");
    };
    assert_eq!(request.image, qemu_model_artifact().identity);
    let session = app.qemu_session(id).expect("session");
    let job_id = session.background_job_id;
    assert_eq!(
        app.background_jobs.get(job_id).map(|job| job.status),
        Some(BackgroundJobStatus::Queued)
    );

    let _ = update(&mut app, Action::BeginSelectedQemuLaunch);
    assert_eq!(
        app.notification.as_deref(),
        Some("A managed runqemu session is already active.")
    );
    let _ = update(
        &mut app,
        Action::QemuSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::QemuSessionRunning { id });
    for index in 0..600 {
        let _ = update(
            &mut app,
            Action::AppendQemuSessionOutput {
                id,
                stream: if index % 2 == 0 {
                    QemuOutputStream::Stdout
                } else {
                    QemuOutputStream::Stderr
                },
                line: format!("line {index}"),
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        );
    }
    let job = app.background_jobs.get(job_id).expect("job");
    assert_eq!(job.status, BackgroundJobStatus::Running);
    assert_eq!(job.output.len(), MAX_BACKGROUND_JOB_OUTPUT_ENTRIES);
    assert_eq!(job.dropped_output_entries, 88);
    assert!(
        job.output
            .iter()
            .any(|entry| entry.source == BackgroundJobOutputSource::Stderr)
    );
}
