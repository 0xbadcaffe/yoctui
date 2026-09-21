use super::*;

#[test]
fn dialog_invalid_actions_leave_active_state_unchanged() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenBuildOptions);
    let original = app.clone();

    assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
    let _ = update(&mut app, Action::AppendBbmask('x'));
    let _ = update(&mut app, Action::CancelImagePicker);

    assert_eq!(app, original);
}
