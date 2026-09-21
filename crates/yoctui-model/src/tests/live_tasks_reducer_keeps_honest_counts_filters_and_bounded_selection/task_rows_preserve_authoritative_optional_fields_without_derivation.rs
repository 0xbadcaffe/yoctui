use super::*;

#[test]
fn task_rows_preserve_authoritative_optional_fields_without_derivation() {
    let mut app = App::new(20, 2_000);
    let mut task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.worker = Some("worker-7".into());
    task.pid = Some(4242);
    task.progress = None;
    app.tasks.insert(task.id.clone(), task);

    let rows = app.visible_task_row_refs_at(SystemTime::UNIX_EPOCH);
    assert!(matches!(
        rows.as_slice(),
        [TaskRowRef::Task { task, state: TaskState::Active }]
            if task.worker.as_deref() == Some("worker-7")
                && task.pid == Some(4242)
                && task.progress.is_none()
    ));
}
