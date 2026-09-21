use super::*;

#[test]
fn task_state_distinguishes_identified_queue_entries_from_aggregate_waiting_work() {
    let mut app = App::new(20, 2_000);
    app.build.status = BuildStatus::Running;
    app.build.total = Some(4);
    let id = TaskId("busybox:do_compile".into());
    let mut queued = TaskInfo::active(id.clone(), "busybox".into(), "do_compile".into());
    queued.pid = Some(4242);

    let _ = update(&mut app, Action::TaskQueued(queued));
    let queued = app.tasks.get(&id).expect("queued task retained");
    assert_eq!(queued.state, TaskState::Queued);
    assert_eq!(queued.started, None, "queued work has not started");
    assert_eq!(queued.pid, None, "queued work cannot retain a running PID");
    assert_eq!(app.waiting_task_count(), 3);
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task), TaskRow::WaitingSummary(3)]
            if task.state == TaskState::Queued
    ));

    app.task_filters.state = TaskStateFilter::Waiting;
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task), TaskRow::WaitingSummary(3)]
            if task.state == TaskState::Queued
    ));

    let started = TaskInfo::active(id.clone(), "busybox".into(), "do_compile".into());
    let _ = update(&mut app, Action::TaskStarted(started));
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::WaitingSummary(3)]
    ));
    app.task_filters.state = TaskStateFilter::Active;
    assert!(matches!(
        app.visible_task_rows().as_slice(),
        [TaskRow::Task(task)] if task.state == TaskState::Active
    ));
}
