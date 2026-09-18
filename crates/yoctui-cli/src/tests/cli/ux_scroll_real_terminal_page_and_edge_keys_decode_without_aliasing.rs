use super::*;

#[test]
fn ux_scroll_real_terminal_page_and_edge_keys_decode_without_aliasing() {
    for (key, input) in [
        (KeyCode::PageUp, Input::PageUp),
        (KeyCode::PageDown, Input::PageDown),
        (KeyCode::Home, Input::Home),
        (KeyCode::End, Input::End),
    ] {
        assert_eq!(
            input_from_key(KeyEvent::new(key, KeyModifiers::NONE)),
            Some(input)
        );
    }
}
