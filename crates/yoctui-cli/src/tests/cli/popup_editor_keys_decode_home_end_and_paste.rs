use super::*;

#[test]
fn popup_editor_keys_decode_home_end_and_paste() {
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)),
        Some(Input::Home)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE)),
        Some(Input::End)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
        Some(Input::PageUp)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
        Some(Input::PageDown)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL)),
        Some(Input::CtrlV)
    );
}
