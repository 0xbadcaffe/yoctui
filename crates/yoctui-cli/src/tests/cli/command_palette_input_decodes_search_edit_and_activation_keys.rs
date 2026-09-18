use super::*;

#[test]
fn command_palette_input_decodes_search_edit_and_activation_keys() {
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)),
        Some(Input::CtrlP)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        Some(Input::Char('x'))
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)),
        Some(Input::Backspace)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(Input::Enter)
    );

    let mut app = App::new(10, 1_000);
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::Char('a'), false),
        Some(Action::OpenContextMenu)
    );
    let _ = update(&mut app, Action::OpenGlobalSearch);
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::Char('a'), false),
        None,
        "the search text owner must receive letters that are also global shortcuts"
    );
    let _ = update(&mut app, Action::AppendCommandPaletteQuery('a'));
    assert_eq!(app.command_palette_query, "a");
}
