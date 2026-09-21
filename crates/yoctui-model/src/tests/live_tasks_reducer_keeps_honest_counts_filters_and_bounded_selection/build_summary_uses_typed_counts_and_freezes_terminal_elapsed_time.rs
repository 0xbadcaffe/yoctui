use super::*;

#[test]
fn build_summary_uses_typed_counts_and_freezes_terminal_elapsed_time() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let mut app = App::new(20, 2_000);
    app.build.status = BuildStatus::Running;
    app.build.started = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(40));
    app.build.completed = 3;
    app.build.total = Some(10);
    app.build.warnings = 2;
    app.build.errors = 1;
    let active = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    app.tasks.insert(active.id.clone(), active);

    let summary = app.build_summary_at(now);
    assert_eq!(summary.completed, 3);
    assert_eq!(summary.total, Some(10));
    assert_eq!(summary.progress_percent(), Some(30));
    assert_eq!(summary.active, 1);
    assert_eq!(summary.waiting, 6);
    assert_eq!(summary.warnings, 2);
    assert_eq!(summary.errors, 1);
    assert_eq!(summary.elapsed, Some(Duration::from_secs(60)));

    app.build.status = BuildStatus::Completed;
    app.build_history.push_back(BuildRecord {
        target: None,
        success: true,
        exit_code: Some(0),
        elapsed: Some(Duration::from_secs(75)),
        completed_tasks: 10,
        warnings: 2,
        errors: 1,
    });
    assert_eq!(
        app.build_summary_at(now + Duration::from_secs(900)).elapsed,
        Some(Duration::from_secs(75))
    );

    app.build.total = None;
    assert_eq!(app.build_summary_at(now).progress_percent(), None);
    app.build.total = Some(0);
    assert_eq!(app.build_summary_at(now).progress_percent(), None);
}
