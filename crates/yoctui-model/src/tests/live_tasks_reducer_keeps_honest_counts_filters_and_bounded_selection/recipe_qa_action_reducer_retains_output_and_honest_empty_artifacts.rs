use super::*;

#[test]
fn recipe_qa_action_reducer_retains_output_and_honest_empty_artifacts() {
    let mut app = App::new(20, 4_000);
    let id = BackgroundJobId(9);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(BackgroundJobSpec {
            id,
            kind: BackgroundJobKind::CveCheck,
            title: "CVE check busybox".into(),
            context: BackgroundJobContext {
                workspace: Some(Screen::Recipes),
                recipe: Some("busybox".into()),
                task: Some("cve_check".into()),
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
                severity: Severity::Warning,
                message: "CVE-2026-0001 requires review".into(),
                source: BackgroundJobOutputSource::Backend,
                truncated: false,
                timestamp: SystemTime::UNIX_EPOCH,
            },
        },
    );
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: BackgroundJobResult {
                summary: "CVE check completed; BitBake reported no result path".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let job = app.background_jobs.get(id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert_eq!(job.warnings, 1);
    assert_eq!(
        job.output.back().unwrap().message,
        "CVE-2026-0001 requires review"
    );
    assert!(job.result.as_ref().unwrap().artifacts.is_empty());
}
