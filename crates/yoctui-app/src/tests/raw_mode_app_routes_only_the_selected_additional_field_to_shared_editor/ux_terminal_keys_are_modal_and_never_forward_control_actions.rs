use super::*;

#[test]
fn ux_terminal_keys_are_modal_and_never_forward_control_actions() {
    let mut app = yoctui_model::App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('o')),
        Some(Action::TerminalTakeControl)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('n')),
        Some(Action::TerminalCreateBuildShell)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('s')),
        Some(Action::TerminalCreateSelectedDevshell)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('m')),
        Some(Action::TerminalCreateSelectedMenuconfig)
    );
    assert_eq!(terminal_workspace_action(&app, Input::Char('a')), None);

    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([3; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 1,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([3; 16]),
            writer_epoch: 1,
            exit_code: None,
            restartable: true,
        });
    app.focus = FocusTarget::Workspace;
    for key in [
        Input::Tab,
        Input::Esc,
        Input::Char('q'),
        Input::Char('a'),
        Input::F12,
    ] {
        assert!(focus_action_for_app(&app, key).is_none());
        assert!(terminal_owns_input(&app));
    }
    for literal in ['n', 's', 'm', 'o', 'r', 'x', '?', '/', 'v'] {
        assert_eq!(
            terminal_workspace_action(&app, Input::Char(literal)),
            None,
            "writer character {literal:?} must remain PTY input"
        );
    }
    assert_eq!(
        terminal_context_action(Input::Char('s')),
        Some(Action::TerminalCreateSelectedDevshell)
    );

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Search;
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('a')),
        Some(Action::TerminalAppendSearch('a'))
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalFinishSearch)
    );

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::PasteReview;
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalConfirmPaste)
    );
    assert_eq!(terminal_workspace_action(&app, Input::Char('y')), None);

    app.terminal.mode = yoctui_model::TerminalWorkbenchMode::KillConfirmation;
    assert_eq!(
        terminal_workspace_action(&app, Input::Enter),
        Some(Action::TerminalConfirmKill)
    );
    assert_eq!(
        terminal_workspace_action(&app, Input::Esc),
        Some(Action::TerminalCancelMode)
    );
}
