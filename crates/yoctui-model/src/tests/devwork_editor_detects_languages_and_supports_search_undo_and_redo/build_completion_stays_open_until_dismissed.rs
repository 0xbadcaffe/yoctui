use super::*;

#[test]
fn build_completion_stays_open_until_dismissed() {
    let mut app = App::new(10, 1_000);
    app.build.target = Some("core-image-minimal".into());
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert!(app.active_dialog().is_none());
}
