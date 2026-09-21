use super::*;

#[test]
fn focus_quit_confirmation_traps_and_restores() {
    let mut app = App::new(10, 1_000);
    app.focus = FocusTarget::Navigator;
    app.build.status = BuildStatus::Running;
    let _ = update(&mut app, Action::Quit);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    assert_eq!(app.focus, FocusTarget::Dialog);

    let _ = update(&mut app, Action::Open(Screen::Logs));
    assert_eq!(app.screen, Screen::Dashboard);
    let _ = update(&mut app, Action::CancelQuit);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, FocusTarget::Navigator);
}
