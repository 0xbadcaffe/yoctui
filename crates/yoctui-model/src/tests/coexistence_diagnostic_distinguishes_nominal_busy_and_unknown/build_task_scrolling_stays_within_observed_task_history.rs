use super::*;

#[test]
fn build_task_scrolling_stays_within_observed_task_history() {
    let mut app = App::new(2, 10);
    for recipe in ["busybox", "bash"] {
        let id = TaskId(format!("{recipe}:do_compile"));
        let _ = update(
            &mut app,
            Action::TaskStarted(TaskInfo {
                id: id.clone(),
                recipe: recipe.into(),
                task: "do_compile".into(),
                progress: None,
                ..TaskInfo::default()
            }),
        );
        let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    }
    let _ = update(&mut app, Action::ScrollBuildTasks { delta: 8 });
    assert_eq!(app.task_progress_scroll, 1);
    let _ = update(&mut app, Action::ScrollBuildTasks { delta: -8 });
    assert_eq!(app.task_progress_scroll, 0);
}
