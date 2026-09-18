use super::*;

#[test]
fn normalizes_task_progress_event() {
    assert!(matches!(
        yoctui_app::model_action_from_backend_event(BackendEvent::TaskProgress {
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: Some(25),
        }),
        Some(Action::TaskProgress {
            progress: Some(25),
            ..
        })
    ));
}
