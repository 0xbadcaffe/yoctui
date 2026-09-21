use super::*;

#[test]
fn beginning_a_build_clears_stale_build_state() {
    let mut app = App::new(10, 1_000);
    app.build.completed = 7;
    app.build.total = Some(10);
    app.build.parse_current = Some(3);
    app.build.parse_total = Some(4);
    app.build.warnings = 2;
    app.build.errors = 1;
    app.build.exit_code = Some(1);
    app.build.started = Some(SystemTime::now());
    app.tasks.insert(
        TaskId("old:task".into()),
        TaskInfo {
            id: TaskId("old:task".into()),
            recipe: "old".into(),
            task: "task".into(),
            progress: Some(50),
            ..TaskInfo::default()
        },
    );
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: None,
        force: false,
    };
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
    assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
    assert_eq!(app.build.target.as_deref(), Some("busybox"));
    assert_eq!(app.build.completed, 0);
    assert_eq!(app.build.total, None);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
    assert_eq!(app.build.warnings, 0);
    assert_eq!(app.build.errors, 0);
    assert_eq!(app.build.exit_code, None);
    assert_eq!(app.build.started, None);
    assert!(app.tasks.is_empty());
}
