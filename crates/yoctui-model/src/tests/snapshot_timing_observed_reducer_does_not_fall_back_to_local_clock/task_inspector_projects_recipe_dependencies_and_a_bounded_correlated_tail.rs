use super::*;

#[test]
fn task_inspector_projects_recipe_dependencies_and_a_bounded_correlated_tail() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: Some("1.36.1".into()),
        ..Recipe::default()
    });
    let mut task = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    task.dependencies = vec![
        TaskId("busybox:do_configure".into()),
        TaskId("virtual/libc:do_populate_sysroot".into()),
    ];
    task.started = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(10));
    app.tasks.insert(task.id.clone(), task);
    for (id, recipe, task, message) in [
        (1, "busybox", "do_compile", "old"),
        (2, "bash", "do_compile", "unrelated"),
        (3, "busybox", "do_compile", "middle"),
        (4, "busybox", "do_compile", "new"),
    ] {
        app.logs.insert(LogEntry {
            id,
            severity: Severity::Info,
            message: message.into(),
            recipe: Some(recipe.into()),
            task: Some(task.into()),
            path: None,
            timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(id),
            build: None,
            protected: false,
            diagnostic: None,
        });
    }
    let rows = app.visible_task_row_refs_at(SystemTime::UNIX_EPOCH + Duration::from_secs(20));
    let inspector = app.task_inspector(rows.first().copied(), 2);
    let TaskInspectorRef::Task {
        task,
        state,
        version,
        revision,
        workdir,
        recent_logs,
    } = inspector
    else {
        panic!("expected selected task inspector")
    };
    assert_eq!(task.recipe, "busybox");
    assert_eq!(state, TaskState::Active);
    assert_eq!(version, Some("1.36.1"));
    assert_eq!(revision, None);
    assert_eq!(workdir, None);
    assert_eq!(task.dependencies.len(), 2);
    assert_eq!(
        recent_logs
            .iter()
            .map(|entry| entry.message.as_str())
            .collect::<Vec<_>>(),
        ["middle", "new"]
    );
    assert_eq!(
        app.task_inspector(Some(TaskRowRef::WaitingSummary(7)), 2),
        TaskInspectorRef::Waiting { count: 7 }
    );
}
