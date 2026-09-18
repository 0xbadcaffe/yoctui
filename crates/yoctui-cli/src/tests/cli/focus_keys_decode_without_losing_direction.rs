use super::*;

#[test]
fn focus_keys_decode_without_losing_direction() {
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
        Some(Input::Tab)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::SHIFT)),
        Some(Input::BackTab)
    );
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Some(Input::Esc)
    );
}
