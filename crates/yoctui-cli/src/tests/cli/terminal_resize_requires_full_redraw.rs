use super::*;

#[test]
fn terminal_resize_requires_full_redraw() {
    assert!(terminal_event_requires_full_redraw(&Event::Resize(238, 57)));
    assert!(!terminal_event_requires_full_redraw(&Event::FocusGained));
    assert!(!terminal_event_requires_full_redraw(&Event::Key(
        KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE,)
    )));
}
