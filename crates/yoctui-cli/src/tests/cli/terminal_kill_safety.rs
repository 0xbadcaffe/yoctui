use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn terminal_kill_safety_cli_traps_prefix_global_writer_and_unknown_raw_keys() {
    let mut app = App::new(8, 1000);
    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::KillConfirmation;
    let route = crate::interactive_runtime::terminal_kill_review_key;
    assert_eq!(
        route(&app, KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        Some(Some(Action::TerminalConfirmKill))
    );
    assert_eq!(
        route(&app, KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        Some(Some(Action::TerminalCancelMode))
    );
    for key in [
        KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::F(4), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::ALT),
        KeyEvent::new(KeyCode::F(11), KeyModifiers::NONE),
    ] {
        assert_eq!(route(&app, key), Some(None), "{key:?}");
    }
    app.terminal.reset_transient_mode();
    assert_eq!(
        route(&app, KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
        None
    );
}
