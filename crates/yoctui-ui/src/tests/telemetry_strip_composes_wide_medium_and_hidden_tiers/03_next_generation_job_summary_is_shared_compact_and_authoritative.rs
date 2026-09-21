#[test]
fn next_generation_job_summary_is_shared_compact_and_authoritative() {
    let mut app = App::new(10, 1_000);
    for (index, status) in [
        BackgroundJobStatus::Queued,
        BackgroundJobStatus::Running,
        BackgroundJobStatus::Failed,
        BackgroundJobStatus::Succeeded,
    ]
    .into_iter()
    .enumerate()
    {
        app.background_jobs
            .jobs
            .push_back(yoctui_model::BackgroundJob {
                id: yoctui_model::BackgroundJobId(index as u64 + 1),
                kind: BackgroundJobKind::Build,
                title: format!("job-{index}"),
                status,
                context: yoctui_model::BackgroundJobContext::default(),
                cancellation_supported: true,
                progress: yoctui_model::BackgroundJobProgress::Indeterminate,
                output: std::collections::VecDeque::new(),
                retained_output_bytes: 0,
                dropped_output_entries: 0,
                warnings: 0,
                errors: usize::from(status == BackgroundJobStatus::Failed),
                queued_at: UNIX_EPOCH,
                started_at: (status != BackgroundJobStatus::Queued)
                    .then_some(UNIX_EPOCH + Duration::from_secs(1)),
                finished_at: status
                    .is_terminal()
                    .then_some(UNIX_EPOCH + Duration::from_secs(2)),
                result: None,
                error: None,
            });
    }
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    for id in 10..12 {
        app.daemon.jobs.push(yoctui_model::ClientDaemonJobSummary {
            id,
            kind: yoctui_model::ClientDaemonJobKind::Utility,
            label: format!("daemon-{id}"),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            progress_current: None,
            progress_total: None,
            exit_code: None,
        });
    }

    let wide = "Active 3 · Queued 1 · Failed 1 · Recent complete 2 · Daemon-owned 2";
    assert_eq!(job_summary_label(&app, 100), wide);
    assert_eq!(job_summary_label(&app, 70), "A3 Q1 F1 Done2 D2");

    app.screen = Screen::Tasks;
    let embedded = rendered_text_at(&app, 180, 44, UNIX_EPOCH + Duration::from_secs(10));
    assert!(embedded.contains(wide), "{embedded}");

    app.screen = Screen::BuildHistory;
    let standalone = rendered_text_at(&app, 180, 34, UNIX_EPOCH + Duration::from_secs(10));
    assert!(standalone.contains("A3 Q1 F1 Done2 D2"), "{standalone}");

    app.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    assert_eq!(job_summary_label(&app, 70), "A1 Q1 F1 Done2");
    assert!(!job_summary_label(&app, 100).contains("Daemon-owned"));
}
