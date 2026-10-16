use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtySummary,
    ClientReplicaStatus, TerminalWorkbenchMode,
};

fn console(writer: Option<[u8; 16]>) -> App {
    let mut app = App::new(8, 1_000);
    app.onboarding.open = false;
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 1,
        name: "shell".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 1,
        kind: ClientDaemonPtyKind::BuildShell,
        cwd: "/build".into(),
        columns: 80,
        rows: 24,
        writer,
        writer_epoch: 3,
        exit_code: None,
        restartable: true,
    });
    app
}

#[test]
fn terminal_search_shortcut_preserves_writer_slashes_and_opens_for_every_viewer() {
    for writer in [Some([7; 16]), Some([8; 16]), None] {
        let app = console(writer);
        assert_eq!(
            terminal_workspace_action(&app, Input::Alt('/')),
            Some(Action::TerminalBeginSearch)
        );
        let bare_slash = terminal_workspace_action(&app, Input::Char('/'));
        if app.selected_terminal_is_writer() {
            assert_eq!(bare_slash, None, "the writer must receive a literal slash");
        } else {
            assert_eq!(bare_slash, Some(Action::TerminalBeginSearch));
        }
        assert_eq!(global_search_action(&app, Input::Char('/')), None);
    }
}

#[test]
fn terminal_search_shortcut_keeps_history_keys_local_and_search_text_literal() {
    let mut app = console(Some([7; 16]));
    app.terminal.mode = TerminalWorkbenchMode::Search;
    for (key, delta) in [
        (Input::Up, 1),
        (Input::Down, -1),
        (Input::PageUp, 10),
        (Input::PageDown, -10),
        (Input::Home, isize::MAX),
        (Input::End, isize::MIN + 1),
    ] {
        assert_eq!(
            terminal_workspace_action(&app, key),
            Some(Action::TerminalScroll { delta })
        );
        assert_eq!(focus_action_for_app(&app, key), None);
    }
    for ch in ['/', 'j', 'k', 'o', '界'] {
        assert_eq!(
            terminal_workspace_action(&app, Input::Char(ch)),
            Some(Action::TerminalAppendSearch(ch))
        );
    }
    assert_eq!(
        terminal_workspace_action(&app, Input::CtrlU),
        Some(Action::TerminalClearSearch)
    );
    for key in [Input::Enter, Input::Esc] {
        assert_eq!(
            terminal_workspace_action(&app, key),
            Some(Action::TerminalFinishSearch)
        );
        assert_eq!(focus_action_for_app(&app, key), None);
    }
}

#[test]
fn terminal_search_shortcut_does_not_escape_browser_or_other_modal_modes() {
    let mut app = console(Some([7; 16]));
    app.terminal.browser_open = true;
    assert_eq!(terminal_workspace_action(&app, Input::Alt('/')), None);
    app.terminal.browser_open = false;
    for mode in [
        TerminalWorkbenchMode::Rename,
        TerminalWorkbenchMode::PasteReview,
        TerminalWorkbenchMode::KillConfirmation,
        TerminalWorkbenchMode::Copy,
    ] {
        app.terminal.mode = mode;
        assert_eq!(
            terminal_workspace_action(&app, Input::Alt('/')),
            None,
            "{mode:?}"
        );
    }
}

#[test]
fn terminal_tree_slash_opens_global_search_without_releasing_the_selected_session() {
    for writer in [Some([7; 16]), Some([8; 16]), None] {
        for focus in [FocusTarget::Workspace, FocusTarget::Navigator] {
            let mut app = console(writer);
            app.focus = focus;
            app.terminal.browser_open = true;
            let action = global_search_action(&app, Input::Char('/'));
            assert_eq!(
                action,
                Some(Action::OpenGlobalSearch),
                "{writer:?} {focus:?}"
            );
            assert_eq!(terminal_workspace_action(&app, Input::Char('/')), None);
            assert!(!terminal_owns_input(&app));
            let _ = compatibility_workspace_action(&mut app, action.unwrap());
            assert!(app.command_palette_open);
            assert_eq!(
                app.command_palette_mode,
                yoctui_model::CommandPaletteMode::GlobalRegexSearch
            );
            assert_eq!(app.daemon.pty_details[0].writer, writer);
            let _ = compatibility_workspace_action(&mut app, Action::CloseCommandPalette);
            assert!(app.terminal_browser_visible());
            assert_eq!(app.focus, focus);
            assert_eq!(
                global_search_action(&app, Input::Char('/')),
                Some(Action::OpenGlobalSearch)
            );
        }
    }
}

#[test]
fn terminal_tree_global_search_supports_empty_tree_and_respects_modals() {
    let mut app = console(None);
    app.terminal.browser_open = true;
    app.daemon.pty_sessions.clear();
    app.daemon.pty_details.clear();
    assert_eq!(
        global_search_action(&app, Input::Char('/')),
        Some(Action::OpenGlobalSearch)
    );
    for mode in [
        TerminalWorkbenchMode::Rename,
        TerminalWorkbenchMode::Search,
        TerminalWorkbenchMode::PasteReview,
        TerminalWorkbenchMode::KillConfirmation,
        TerminalWorkbenchMode::Help,
        TerminalWorkbenchMode::Copy,
    ] {
        app.terminal.mode = mode;
        assert_eq!(
            global_search_action(&app, Input::Char('/')),
            None,
            "{mode:?}"
        );
    }
    app.terminal.mode = TerminalWorkbenchMode::Live;
    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    assert_eq!(global_search_action(&app, Input::Char('/')), None);
}
