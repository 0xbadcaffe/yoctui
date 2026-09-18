use super::*;

#[test]
fn reference_function_keys_decode_without_aliasing() {
    let expected = [
        Input::F1,
        Input::F2,
        Input::F3,
        Input::F4,
        Input::F5,
        Input::F6,
        Input::F7,
        Input::F8,
        Input::F9,
        Input::F10,
    ];
    for (number, expected) in (1..=10).zip(expected) {
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::F(number), KeyModifiers::NONE)),
            Some(expected)
        );
    }
}
