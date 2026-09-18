use super::*;

#[test]
fn search_clear_control_key_decodes_without_becoming_text() {
    let key = KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(input_from_key(key), Some(Input::CtrlU));
}
