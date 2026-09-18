use super::*;

#[test]
fn dialog_input_routing_prevents_pane_shortcuts_from_leaking() {
    let mut app = App::new(10, 1_000);
    app.focus = yoctui_model::FocusTarget::Inspector;
    let _ = update(&mut app, Action::OpenBuildOptions);
    let selection = app.navigator_selection;

    let tab = input_from_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)).unwrap();
    assert_eq!(yoctui_app::focus_action(app.focus, tab), None);
    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });

    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    assert_eq!(app.navigator_selection, selection);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
}
