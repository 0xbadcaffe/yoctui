use super::*;

#[test]
fn devtool_job_lifecycle_retains_typed_output_and_outcome_across_navigation() {
    let mut app = App::new(10, 1_000);
    let id = BackgroundJobId(1_u64 << 63);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::Devtool,
            title: "Devtool reset busybox".into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("busybox".into()),
                ..BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::AppendBackgroundJobOutput {
            id,
            entry: BackgroundJobOutputEntry {
                severity: Severity::Info,
                message: "workspace reset".into(),
                source: BackgroundJobOutputSource::Stderr,
                truncated: true,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(&mut app, Action::Open(Screen::Dashboard));
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "Devtool completed successfully".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.context.recipe.as_deref(), Some("busybox"));
    assert_eq!(job.output[0].source, BackgroundJobOutputSource::Stderr);
    assert!(job.output[0].truncated);
    assert_eq!(app.screen, Screen::Dashboard);
}
