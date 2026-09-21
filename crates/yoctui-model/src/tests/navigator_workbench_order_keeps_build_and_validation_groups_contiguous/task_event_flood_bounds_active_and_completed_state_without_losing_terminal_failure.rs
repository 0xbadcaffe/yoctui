use super::*;

#[test]
fn task_event_flood_bounds_active_and_completed_state_without_losing_terminal_failure() {
    let mut app = App::new(16, 4096);
    let mut events = (0..MAX_ACTIVE_TASKS + 128)
        .map(|index| {
            let id = TaskId(format!("recipe-{index}:do_compile"));
            TaskEvent::Started(TaskInfo {
                id,
                recipe: format!("recipe-{index}"),
                task: "do_compile".into(),
                ..TaskInfo::default()
            })
        })
        .collect::<Vec<_>>();
    let overflow_id = TaskId(format!("recipe-{}:do_compile", MAX_ACTIVE_TASKS + 127));
    events.push(TaskEvent::Completed {
        id: overflow_id.clone(),
        success: false,
    });
    let _ = update(&mut app, Action::TaskEvents(events));
    assert_eq!(app.tasks.len(), MAX_ACTIVE_TASKS);
    assert_eq!(app.task_active_overflow, 128);
    assert!(app.completed_tasks.len() <= MAX_COMPLETED_TASKS);
    assert!(app.completed_tasks.iter().any(|completed| {
        completed.task.id == overflow_id
            && !completed.success
            && completed.task.state == TaskState::Failed
    }));
}
