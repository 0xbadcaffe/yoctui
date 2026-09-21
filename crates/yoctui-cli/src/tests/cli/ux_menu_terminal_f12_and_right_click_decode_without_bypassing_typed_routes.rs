use super::*;

#[test]
fn ux_menu_terminal_f12_and_right_click_decode_without_bypassing_typed_routes() {
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::F(12), KeyModifiers::NONE)),
        Some(Input::F12)
    );
    assert_eq!(
        mouse_kind_from_event(crossterm::event::MouseEventKind::Down(
            crossterm::event::MouseButton::Right,
        )),
        Some(MouseKind::ContextDown)
    );
    assert_eq!(
        mouse_kind_from_event(crossterm::event::MouseEventKind::Down(
            crossterm::event::MouseButton::Left,
        )),
        Some(MouseKind::Down)
    );

    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::OpenApplicationMenu);
    assert!(app.menu.is_open());
    assert!(matches!(
        menu_action(&app, Input::Esc),
        Some(MenuInputResult::Reduce(action)) if *action == Action::CloseMenu
    ));
}
