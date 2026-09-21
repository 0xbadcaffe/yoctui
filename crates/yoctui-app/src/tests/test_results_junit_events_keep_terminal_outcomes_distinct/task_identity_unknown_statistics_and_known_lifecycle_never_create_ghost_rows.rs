use super::*;

#[test]
fn task_identity_unknown_statistics_and_known_lifecycle_never_create_ghost_rows() {
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    let _ = yoctui_model::update(
        &mut app,
        Action::BuildRequested {
            target: Some("image".into()),
        },
    );
    for event in [
        BackendEvent::BuildStarted,
        BackendEvent::TaskStats(yoctui_model::TaskStats {
            completed: 3,
            total: 10,
            active: 1,
            failed: 0,
        }),
    ] {
        let _ = yoctui_model::update(&mut app, model_action_from_backend_event(event).unwrap());
    }
    assert_eq!((app.build.completed, app.build.total), (3, Some(10)));
    assert!(app.tasks.is_empty());
    for recipe in ["llvm-native", "lib32-actual-name", "overridden-name"] {
        for event in [
            BackendEvent::TaskQueued {
                recipe: recipe.into(),
                task: "do_compile".into(),
                worker: None,
                stats: None,
            },
            BackendEvent::TaskStarted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                worker: None,
                stats: None,
                pid: Some(42),
                log_path: None,
            },
            BackendEvent::TaskCompleted {
                recipe: recipe.into(),
                task: "do_compile".into(),
                success: true,
            },
        ] {
            let _ = yoctui_model::update(&mut app, model_action_from_backend_event(event).unwrap());
        }
        assert!(app.tasks.is_empty());
    }
    assert_eq!(app.completed_tasks.len(), 3);
    assert_eq!(app.build.completed, 6);
}
