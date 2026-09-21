use super::*;

#[test]
fn focus_restores_exact_pane_after_nested_dialog_transitions() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;

    let _ = update(&mut app, Action::OpenBuildOptions);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Workspace));

    let _ = update(&mut app, Action::BeginBuildTargetEdit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildTarget { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Workspace));

    let _ = update(&mut app, Action::CancelBuildTargetEdit);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.focus_return, None);
}
