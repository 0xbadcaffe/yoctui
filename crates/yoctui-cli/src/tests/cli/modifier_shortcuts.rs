use super::*;

#[test]
fn modifier_terminal_keys_decode_and_forward_without_losing_alt_or_text() {
    for character in ['f', 'g', 'w', 'F', 'G'] {
        let key = KeyEvent::new(KeyCode::Char(character), KeyModifiers::ALT);
        assert_eq!(input_from_key(key), Some(Input::Alt(character)));
        assert_eq!(
            terminal_input_bytes(Input::Alt(character)),
            Some(format!("\x1b{character}").into_bytes())
        );
        assert_eq!(
            terminal_key_bytes(key, false),
            Some(format!("\x1b{character}").into_bytes())
        );
        assert_eq!(
            input_from_key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)),
            Some(Input::Char(character))
        );
    }
    assert_eq!(
        input_from_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)),
        Some(Input::CtrlF)
    );
    for character in ['f', 'F'] {
        assert_eq!(
            input_from_key(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            Some(Input::CtrlShiftF)
        );
    }
    assert_eq!(
        input_from_key(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::ALT | KeyModifiers::CONTROL
        )),
        Some(Input::Alt('\x07'))
    );
    for (character, modifiers) in [
        ('.', KeyModifiers::ALT),
        ('é', KeyModifiers::ALT),
        ('g', KeyModifiers::ALT | KeyModifiers::CONTROL),
    ] {
        let event = KeyEvent::new(KeyCode::Char(character), modifiers);
        assert_eq!(
            terminal_input_bytes(input_from_key(event).unwrap()),
            terminal_key_bytes(event, false)
        );
    }
    let mut released = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::ALT);
    released.kind = crossterm::event::KeyEventKind::Release;
    assert_eq!(input_from_key(released), None);
}
