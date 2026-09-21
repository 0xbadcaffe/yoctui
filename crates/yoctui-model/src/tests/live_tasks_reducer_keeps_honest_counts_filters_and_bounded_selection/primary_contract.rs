use super::*;

#[test]
fn live_tasks_reducer_keeps_honest_counts_filters_and_bounded_selection() {
    let mut app = App::new(20, 2_000);
    let first = TaskId("busybox:do_compile".into());
    let second = TaskId("openssl:do_install".into());
    let mut busybox = TaskInfo::active(first.clone(), "busybox".into(), "do_compile".into());
    busybox.worker = Some("worker-1".into());
    busybox.stats = Some(TaskStats {
        completed: 1,
        total: 5,
        active: 1,
        failed: 0,
    });
    let _ = update(&mut app, Action::TaskStarted(busybox));
    let mut openssl = TaskInfo::active(second.clone(), "openssl".into(), "do_install".into());
    openssl.worker = Some("worker-2".into());
    let _ = update(&mut app, Action::TaskStarted(openssl));
    assert_eq!(app.build.completed, 1);
    assert_eq!(app.build.total, Some(5));
    assert_eq!(app.waiting_task_count(), 2);
    assert!(matches!(
        app.visible_task_rows().last(),
        Some(TaskRow::WaitingSummary(2))
    ));

    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: second,
            success: false,
        },
    );
    let _ = update(&mut app, Action::CycleTaskStateFilter);
    assert_eq!(app.task_filters.state, TaskStateFilter::Active);
    assert_eq!(app.visible_task_rows().len(), 1);
    for _ in 0..3 {
        let _ = update(&mut app, Action::CycleTaskStateFilter);
    }
    assert_eq!(app.task_filters.state, TaskStateFilter::Failed);
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task)] if task.recipe == "openssl" && task.state == TaskState::Failed
    ));

    app.task_progress_scroll = 99;
    let _ = update(&mut app, Action::CycleTaskDurationFilter);
    assert_eq!(app.task_progress_scroll, 0);
    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: first,
            success: true,
        },
    );
    assert!(app.task_progress_scroll <= app.visible_task_rows().len().saturating_sub(1));
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(app.build.completed, 5);
    assert_eq!(app.waiting_task_count(), 0);
}
