use super::*;

#[test]
fn ux_dashboard_prioritizes_failures_then_active_work_without_duplicating_state() {
    let mut app = App::new(16, 4_096);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.source_dir = Some("/work/poky".into());
    app.workspace.build_dir = Some("/work/poky/build".into());
    app.build.status = BuildStatus::Running;
    app.tasks.insert(
        TaskId("busybox:do_compile".into()),
        TaskInfo::active(
            TaskId("busybox:do_compile".into()),
            "busybox".into(),
            "do_compile".into(),
        ),
    );
    let running = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(
        running.next_action.kind,
        DashboardNextActionKind::MonitorTasks
    );
    assert_eq!(running.summary.active, 1);
    assert_eq!(running.health.environment, DashboardEnvironmentState::Ready);

    for index in 1..=6 {
        let _ = update(
            &mut app,
            Action::Log(log(
                if index == 5 {
                    Severity::Error
                } else {
                    Severity::Warning
                },
                index,
            )),
        );
    }
    let failed = app.dashboard_projection_at(SystemTime::UNIX_EPOCH);
    assert_eq!(
        failed.next_action.kind,
        DashboardNextActionKind::ReviewFailures
    );
    assert_eq!(failed.failures.len(), DASHBOARD_COLLECTION_LIMIT);
    assert_eq!(failed.failures[0].message, "diagnostic-6");
}
