use super::*;

#[test]
fn task_activity_transitions_parsing_to_running_without_regression() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::BuildRequested {
            target: Some("core-image-minimal".into()),
        },
    );
    let _ = update(&mut app, Action::BuildStarted);
    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(59),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Parsing);

    let task = TaskInfo {
        id: TaskId("glibc:do_compile".into()),
        recipe: "glibc".into(),
        task: "do_compile".into(),
        stats: Some(TaskStats {
            completed: 59,
            total: 100,
            active: 1,
            failed: 0,
        }),
        ..TaskInfo::default()
    };
    let _ = update(&mut app, Action::TaskQueued(task));
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);

    let _ = update(
        &mut app,
        Action::ParseProgress {
            current: Some(100),
            total: Some(100),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Running);
    assert_eq!(app.build.parse_current, None);
    assert_eq!(app.build.parse_total, None);
}
