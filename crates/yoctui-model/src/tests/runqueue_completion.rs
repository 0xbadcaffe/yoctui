use super::*;

fn queued(recipe: &str, task: &str) -> TaskEvent {
    TaskEvent::Queued(TaskInfo {
        id: TaskId(format!("{recipe}:{task}")),
        recipe: recipe.into(),
        task: task.into(),
        ..TaskInfo::default()
    })
}

#[test]
fn runqueue_completion_finishes_noexec_without_worker_and_counts_once() {
    let mut app = App::new(16, 4096);
    app.build.status = BuildStatus::Running;
    app.build.total = Some(6454);
    app.build.completed = 6453;
    let completed = TaskEvent::Completed {
        id: TaskId("obmc-phosphor-image:do_build".into()),
        success: true,
    };
    update(
        &mut app,
        Action::TaskEvents(vec![
            queued("obmc-phosphor-image", "do_build"),
            completed.clone(),
            completed,
        ]),
    );
    assert!(app.tasks.is_empty());
    assert_eq!(app.build.completed, 6454);
    assert_eq!(app.completed_tasks.len(), 1);
    let terminal = &app.completed_tasks[0];
    assert!(terminal.success);
    assert_eq!(terminal.task.state, TaskState::Completed);
    assert_eq!(terminal.task.progress, Some(100));
    assert_eq!(terminal.task.pid, None);
    assert_eq!(terminal.task.started, None);
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Completed);
    assert_eq!(app.completed_tasks[0].task.state, TaskState::Completed);
    assert_eq!(app.build.completed, 6454);
}

#[test]
fn runqueue_completion_duplicates_do_not_change_worker_finish_or_count() {
    let mut app = App::new(16, 4096);
    let id = TaskId("busybox:do_compile".into());
    let completion = TaskEvent::Completed {
        id: id.clone(),
        success: true,
    };
    update(
        &mut app,
        Action::TaskEvents(vec![
            TaskEvent::Started(TaskInfo::active(id, "busybox".into(), "do_compile".into())),
            completion.clone(),
        ]),
    );
    let first = app.completed_tasks[0].clone();
    update(&mut app, Action::TaskEvents(vec![completion]));
    assert_eq!(app.completed_tasks.len(), 1);
    assert_eq!(app.completed_tasks[0], first);
    assert_eq!(app.build.completed, 1);
}

#[test]
fn runqueue_completion_does_not_promote_failures_or_unobserved_tasks() {
    let mut app = App::new(16, 4096);
    app.build.status = BuildStatus::Running;
    update(
        &mut app,
        Action::TaskEvents(vec![
            queued("busybox", "do_compile"),
            TaskEvent::Completed {
                id: TaskId("busybox:do_compile".into()),
                success: false,
            },
            queued("unknown-outcome", "do_build"),
        ]),
    );
    update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert_eq!(app.completed_tasks.len(), 2);
    assert_eq!(app.completed_tasks[0].task.state, TaskState::Failed);
    assert_eq!(app.completed_tasks[1].task.state, TaskState::Lost);
    assert!(app.completed_tasks.iter().all(|task| !task.success));
}
