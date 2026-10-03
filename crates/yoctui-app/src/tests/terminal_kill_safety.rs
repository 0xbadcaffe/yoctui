use super::*;
use yoctui_model::{App, TerminalWorkbenchMode};

#[test]
fn terminal_kill_safety_only_enter_escape_act_even_for_unfocused_writer() {
    let mut app = App::new(8, 1000);
    app.terminal.mode = TerminalWorkbenchMode::KillConfirmation;
    app.focus = FocusTarget::Navigator;
    assert!(terminal_owns_input(&app));
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalConfirmKill)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Esc),
        Some(Action::TerminalCancelMode)
    );
    for input in [
        Input::Char('q'),
        Input::Char('x'),
        Input::Char('n'),
        Input::Char('o'),
        Input::Tab,
        Input::BackTab,
        Input::Up,
        Input::Down,
        Input::F4,
        Input::F12,
        Input::CtrlB,
        Input::CtrlP,
        Input::CtrlV,
        Input::Alt('o'),
    ] {
        assert_eq!(terminal_workspace_action(&app, input), None, "{input:?}");
        assert_eq!(focus_action_for_app(&app, input), None, "{input:?}");
    }
}

#[test]
fn terminal_kill_safety_mouse_cannot_change_focus_selection_or_open_menu() {
    let mut app = App::new(8, 1000);
    app.screen = Screen::TerminalSessions;
    app.terminal.mode = TerminalWorkbenchMode::KillConfirmation;
    for kind in [
        MouseKind::Down,
        MouseKind::ContextDown,
        MouseKind::ScrollUp,
        MouseKind::ScrollDown,
    ] {
        for (column, row) in [(12, 8), (55, 9), (120, 20)] {
            assert_eq!(
                mouse_action_for_app(MouseInput { kind, column, row }, &app, 160, 50),
                None
            );
        }
    }
}
