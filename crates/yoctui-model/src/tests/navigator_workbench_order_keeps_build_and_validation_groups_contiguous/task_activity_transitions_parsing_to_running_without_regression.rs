use super::*;

#[test]
fn disk_guard_failed_completion_keeps_partial_progress_and_failed_history() {
    let mut app = App::new(10, 1_000);
    app.build.status = BuildStatus::Running;
    app.build.target = Some("petalinux-image-minimal".into());
    app.build.completed = 3_092;
    app.build.total = Some(10_994);
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Failed);
    assert_eq!(app.build.completed, 3_092);
    assert_eq!(app.build.total, Some(10_994));
    let history = app.build_history.back().unwrap();
    assert!(!history.success);
    assert_eq!(history.completed_tasks, 3_092);
    assert_eq!(history.exit_code, Some(1));
}

#[test]
fn task_activity_transitions_parsing_to_running_without_regression() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::BuildRequested {
            target: Some("core-image-minimal".into()),
        },
    );
    let _ = update(&mut app, Action::BuildStarted);
    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(59),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Parsing);

    let task = TaskInfo {
        id: TaskId("glibc:do_compile".into()),
        recipe: "glibc".into(),
        task: "do_compile".into(),
        stats: Some(TaskStats {
            completed: 59,
            total: 100,
            active: 1,
            failed: 0,
        }),
        ..TaskInfo::default()
    };
    let _ = update(&mut app, Action::TaskQueued(task));
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);

    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(100),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
}
