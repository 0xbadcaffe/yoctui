use yoctui_model::{Action, App, TaskStats, update};

#[test]
fn task_identity_statistics_are_monotonic_and_never_invent_active_tasks() {
    let mut app = App::new(64, 64 * 1024);
    let _ = update(&mut app, Action::BuildStarted);
    for completed in [20, 10] {
        let _ = update(
            &mut app,
            Action::TaskStats(TaskStats {
                completed,
                total: 100,
                active: 7,
                failed: 0,
            }),
        );
    }
    assert_eq!((app.build.completed, app.build.total), (20, Some(100)));
    assert!(app.tasks.is_empty());
    assert!(app.completed_tasks.is_empty());
    let _ = update(
        &mut app,
        Action::TaskStats(TaskStats {
            completed: 20,
            total: 0,
            active: 0,
            failed: 0,
        }),
    );
    assert_eq!(app.build.total, None);
    let _ = update(
        &mut app,
        Action::BuildRequested {
            target: Some("next".into()),
        },
    );
    assert_eq!((app.build.completed, app.build.total), (0, None));
}
