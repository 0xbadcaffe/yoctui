use super::*;

#[test]
fn task_batches_coalesce_progress_and_preserve_terminal_failures() {
    let mut app = App::new(16, 4096);
    let failed = TaskId("busybox:do_compile".into());
    let succeeded = TaskId("base-files:do_install".into());
    let task = |id: &TaskId| TaskInfo {
        id: id.clone(),
        recipe: id.0.split(':').next().unwrap().into(),
        task: id.0.split(':').nth(1).unwrap().into(),
        ..TaskInfo::default()
    };
    let _ = update(
        &mut app,
        Action::TaskEvents(vec![
            TaskEvent::Started(task(&failed)),
            TaskEvent::Progress {
                id: failed.clone(),
                progress: Some(10),
            },
            TaskEvent::Progress {
                id: failed.clone(),
                progress: Some(90),
            },
            TaskEvent::Started(task(&succeeded)),
            TaskEvent::Progress {
                id: succeeded.clone(),
                progress: Some(40),
            },
            TaskEvent::Completed {
                id: failed,
                success: false,
            },
            TaskEvent::Completed {
                id: succeeded,
                success: true,
            },
        ]),
    );
    assert_eq!(app.task_progress_events, 3);
    assert_eq!(app.task_progress_coalesced, 1);
    assert!(app.tasks.is_empty());
    assert_eq!(app.completed_tasks.len(), 2);
    assert!(!app.completed_tasks[0].success);
    assert_eq!(app.completed_tasks[0].task.state, TaskState::Failed);
    assert!(app.completed_tasks[1].success);
    assert_eq!(app.build.completed, 2);
}
