use super::*;

#[test]
fn live_tasks_filter_supports_recipe_task_worker_and_duration() {
    let mut app = App::new(20, 2_000);
    let mut task = TaskInfo::active(
        TaskId("linux-yocto:do_compile_kernel".into()),
        "linux-yocto".into(),
        "do_compile_kernel".into(),
    );
    task.worker = Some("remote-7".into());
    task.started = Some(SystemTime::UNIX_EPOCH);
    task.finished = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(20));
    task.state = TaskState::Completed;
    app.completed_tasks.push_back(CompletedTask {
        task,
        success: true,
    });
    app.task_filters.recipe = "LINUX".into();
    app.task_filters.task = "kernel".into();
    app.task_filters.worker = "REMOTE".into();
    app.task_filters.minimum_duration = Some(Duration::from_secs(10));
    assert_eq!(app.visible_task_rows().len(), 1);
    app.task_filters.minimum_duration = Some(Duration::from_secs(60));
    assert!(app.visible_task_rows().is_empty());
}
