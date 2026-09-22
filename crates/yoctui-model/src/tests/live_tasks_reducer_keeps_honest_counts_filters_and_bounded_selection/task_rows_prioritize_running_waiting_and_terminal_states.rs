use super::*;

#[test]
fn task_rows_prioritize_running_waiting_and_terminal_states() {
    let mut app = App::new(20, 2_000);
    app.build.total = Some(4);
    app.build.completed = 1;

    let completed_id = TaskId("old:do_compile".into());
    let completed = TaskInfo::active(completed_id.clone(), "old".into(), "do_compile".into());
    let _ = update(&mut app, Action::TaskStarted(completed));
    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: completed_id,
            success: true,
        },
    );

    let queued = TaskInfo::active(
        TaskId("next:do_fetch".into()),
        "next".into(),
        "do_fetch".into(),
    );
    let _ = update(&mut app, Action::TaskQueued(queued));
    let running = TaskInfo::active(
        TaskId("current:do_install".into()),
        "current".into(),
        "do_install".into(),
    );
    let _ = update(&mut app, Action::TaskStarted(running));
    app.build.total = Some(6);

    let rows = app.visible_task_rows();
    assert!(matches!(&rows[0], TaskRow::Task(task) if task.state == TaskState::Active));
    assert!(matches!(&rows[1], TaskRow::Task(task) if task.state == TaskState::Queued));
    assert!(matches!(&rows[2], TaskRow::WaitingSummary(_)));
    assert!(matches!(&rows[3], TaskRow::Task(task) if task.state == TaskState::Completed));
}
