use super::*;

#[test]
fn background_job_history_pins_active_rows_and_bounds_shared_selection() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: BackgroundJobId(1),
            result: BackgroundJobResult {
                summary: "done".into(),
                artifacts: Vec::new(),
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(5),
        },
    );
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(2, true)),
    );
    app.build_history.push_back(BuildRecord {
        target: Some("core-image-minimal".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(9)),
        completed_tasks: 4,
        warnings: 0,
        errors: 0,
    });

    let rows = app.job_history_rows();
    assert!(matches!(
        rows.as_slice(),
        [
            JobHistoryRowRef::Background(active),
            JobHistoryRowRef::Background(terminal),
            JobHistoryRowRef::Build(_)
        ] if active.id == BackgroundJobId(2)
            && active.status == BackgroundJobStatus::Queued
            && terminal.id == BackgroundJobId(1)
            && terminal.status == BackgroundJobStatus::Succeeded
    ));
    let _ = update(&mut app, Action::SelectBuildHistory { delta: 99 });
    assert_eq!(app.build_history_selection, 2);
}
