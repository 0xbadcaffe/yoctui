use super::*;

#[test]
fn ctrl_c_is_not_the_regular_cancel_key() {
    let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(input_from_key(key), Some(Input::CtrlC));
}
