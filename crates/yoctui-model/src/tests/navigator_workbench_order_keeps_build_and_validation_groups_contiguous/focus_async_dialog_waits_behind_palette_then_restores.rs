use super::*;

#[test]
fn focus_async_dialog_waits_behind_palette_then_restores() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let _ = update(&mut app, Action::OpenCommandPalette);
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: true,
            exit_code: Some(0),
        },
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildCompletion)));
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    let _ = update(&mut app, Action::CloseCommandPalette);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let _ = update(&mut app, Action::DismissBuildCompletion);
    assert_eq!(app.focus, FocusTarget::Workspace);
}
