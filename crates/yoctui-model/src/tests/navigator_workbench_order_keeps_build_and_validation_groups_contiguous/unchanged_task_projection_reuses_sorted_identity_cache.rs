use super::*;

#[test]
fn unchanged_task_projection_reuses_sorted_identity_cache() {
    let mut app = App::new(16, 4096);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            ..TaskInfo::default()
        }),
    );
    let rebuilds = app.task_projection_rebuilds();
    assert_eq!(app.visible_task_row_refs_at(SystemTime::now()).len(), 1);
    assert_eq!(app.task_projection_rebuilds(), rebuilds);
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id,
            progress: Some(75),
        },
    );
    let rows = app.visible_task_row_refs_at(SystemTime::now());
    assert!(matches!(rows[0], TaskRowRef::Task { task, .. } if task.progress == Some(75)));
    assert_eq!(app.task_projection_rebuilds(), rebuilds);
    let _ = update(&mut app, Action::CycleTaskStateFilter);
    assert!(app.task_projection_rebuilds() > rebuilds);
}
