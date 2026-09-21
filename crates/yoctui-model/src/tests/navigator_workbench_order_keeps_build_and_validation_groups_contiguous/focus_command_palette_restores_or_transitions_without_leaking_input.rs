use super::*;

#[test]
fn focus_command_palette_restores_or_transitions_without_leaking_input() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    app.workspace.build_dir = Some(PathBuf::from("/build"));

    let _ = update(&mut app, Action::OpenCommandPalette);
    assert_eq!(app.focus, FocusTarget::CommandPalette);
    assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
    for character in "Choose theme".chars() {
        let _ = update(&mut app, Action::AppendCommandPaletteQuery(character));
    }

    let original_screen = app.screen;
    let original_selection = app.navigator_selection;
    let _ = update(&mut app, Action::Open(Screen::Logs));
    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    let _ = update(&mut app, Action::Focus(FocusTarget::Workspace));
    assert_eq!(app.screen, original_screen);
    assert_eq!(app.navigator_selection, original_selection);
    assert_eq!(app.focus, FocusTarget::CommandPalette);

    let _ = update(&mut app, Action::ActivateCommandPalette);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ThemePicker { .. })
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.focus_return, Some(FocusTarget::Navigator));
    let _ = update(&mut app, Action::CloseThemePicker);
    assert_eq!(app.focus, FocusTarget::Navigator);
}
