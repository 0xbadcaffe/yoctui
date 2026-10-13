use super::*;
use yoctui_model::{App, TerminalWorkbenchMode};

#[test]
fn terminal_browser_keyboard_routes_navigation_and_open_not_terminal_input() {
    let mut app = App::new(8, 1000);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.terminal.browser_open = true;
    for (key, delta) in [
        (Input::Down, 1),
        (Input::Up, -1),
        (Input::PageDown, 10),
        (Input::End, isize::MAX),
        (Input::Home, isize::MIN),
    ] {
        assert_eq!(
            terminal_workspace_action(&app, key),
            Some(Action::SelectPtySession { delta })
        );
    }
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalTakeControl)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Alt('o')),
        Some(Action::TerminalReleaseControl)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('n')),
        Some(Action::TerminalCreateBuildShell)
    );
    assert_eq!(terminal_workspace_action(&app, Input::Char('x')), None);
    assert!(!terminal_owns_input(&app));
    app.terminal.mode = TerminalWorkbenchMode::Rename;
    assert!(terminal_owns_input(&app));
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('x')),
        Some(Action::TerminalAppendRename('x'))
    );
}

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
