use super::*;

#[test]
fn background_job_completes_and_survives_workspace_navigation() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(background_job_spec(1, true)),
    );
    let _ = update(&mut app, Action::Open(Screen::Layers));
    run_background_job(&mut app, 1);
    let _ = update(
        &mut app,
        Action::UpdateBackgroundJobProgress {
            id,
            progress: BackgroundJobProgress::Units {
                completed: 4,
                total: 10,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id,
            entry: BackgroundJobOutputEntry {
                severity: Severity::Warning,
                message: "cache miss".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(2),
            },
        },
    );
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "image built".into(),
                artifacts: vec!["/deploy/core-image-minimal.wic".into()],
            },
            finished_at: SystemTime::UNIX_EPOCH + Duration::from_secs(3),
        },
    );
    let _ = update(&mut app, Action::Open(Screen::Settings));

    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(app.screen, Screen::Settings);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(
        job.progress,
        BackgroundJobProgress::Units {
            completed: 4,
            total: 10
        }
    );
    assert_eq!(job.warnings, 1);
    assert_eq!(
        job.started_at,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1))
    );
    assert_eq!(
        job.finished_at,
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(3))
    );
    assert_eq!(
        job.result.as_ref().map(|result| result.summary.as_str()),
        Some("image built")
    );
}
