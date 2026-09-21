use super::*;

#[test]
fn ux_progress_preserves_unknown_totals_and_terminal_job_progress() {
    let mut app = App::new(16, 4_096);
    app.build.status = BuildStatus::Running;
    app.build.completed = 12;
    let hierarchy = app.progress_hierarchy_at(SystemTime::UNIX_EPOCH);
    assert_eq!(hierarchy.build.state, WidgetState::Active);
    assert!(hierarchy.build.text(false, true).contains("12/?"));
    assert!(
        hierarchy
            .runqueue
            .text(false, true)
            .contains("progress unknown")
    );

    let job = BackgroundJob {
        id: BackgroundJobId(1),
        kind: BackgroundJobKind::Build,
        title: "SDK".into(),
        status: BackgroundJobStatus::Failed,
        context: BackgroundJobContext::default(),
        cancellation_supported: true,
        progress: BackgroundJobProgress::Units {
            completed: 7,
            total: 10,
        },
        output: VecDeque::new(),
        retained_output_bytes: 0,
        dropped_output_entries: 0,
        warnings: 0,
        errors: 1,
        queued_at: SystemTime::UNIX_EPOCH,
        started_at: Some(SystemTime::UNIX_EPOCH),
        finished_at: Some(SystemTime::UNIX_EPOCH),
        result: None,
        error: None,
    };
    let projection = job.progress_projection();
    assert_eq!(projection.state, WidgetState::TerminalFailure);
    assert_eq!(projection.fraction.unwrap().exact_text(), "7/10 (70%)");
}
