use super::*;

#[test]
fn log_terminal_diagnostics_are_protected_and_observable() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    let entry = app.logs.entries.back().unwrap();
    assert!(entry.protected);
    assert_eq!(entry.build.as_deref(), Some("core-image-minimal"));
    assert!(entry.message.contains("completed"));
}
