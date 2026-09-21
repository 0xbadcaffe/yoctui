use super::*;

#[test]
fn duplicate_or_unknown_completion_does_not_increment_task_count() {
    let mut app = App::new(2, 10);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskCompleted {
            id: id.clone(),
            success: true,
        },
    );
    let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    assert_eq!(app.build.completed, 1);
    assert_eq!(app.completed_tasks.len(), 1);
    assert!(app.completed_tasks.front().is_some_and(|task| task.success));
}
