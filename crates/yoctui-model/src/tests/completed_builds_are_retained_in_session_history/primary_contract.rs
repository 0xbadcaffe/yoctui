use super::*;

#[test]
fn completed_builds_are_retained_in_session_history() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    app.build.completed = 12;
    app.build.warnings = 2;
    app.build.errors = 1;
    app.build.started = Some(SystemTime::now());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert_eq!(app.build_history.len(), 1);
    assert_eq!(
        app.build_history[0].target.as_deref(),
        Some("core-image-minimal")
    );
    assert!(!app.build_history[0].success);
    assert_eq!(app.build_history[0].completed_tasks, 12);
    assert_eq!(app.build_history[0].errors, 1);
}
