use super::*;

#[test]
fn live_tasks_input_maps_selection_and_filter_controls() {
    assert_eq!(
        tasks_action(false, Input::Down),
        Some(Action::ScrollBuildTasks { delta: 1 })
    );
    assert_eq!(
        tasks_action(false, Input::Char('f')),
        Some(Action::CycleTaskStateFilter)
    );
    assert_eq!(
        tasks_action(false, Input::Char('F')),
        Some(Action::CycleTaskFilterField)
    );
    assert_eq!(
        tasks_action(false, Input::Char('/')),
        Some(Action::BeginTaskFilterEdit)
    );
    assert_eq!(
        tasks_action(true, Input::Char('x')),
        Some(Action::AppendTaskFilter('x'))
    );
    assert_eq!(
        tasks_action(true, Input::Esc),
        Some(Action::FinishTaskFilterEdit)
    );
    let action = model_action_from_backend_event(BackendEvent::TaskQueued {
        recipe: "busybox".into(),
        task: "do_compile".into(),
        worker: Some("worker-1".into()),
        stats: Some(yoctui_model::TaskStats {
            completed: 3,
            total: 10,
            active: 2,
            failed: 0,
        }),
    });
    assert!(matches!(
        action,
        Some(Action::TaskQueued(TaskInfo {
            worker: Some(worker),
            stats: Some(yoctui_model::TaskStats { total: 10, .. }),
            ..
        })) if worker == "worker-1"
    ));
}
