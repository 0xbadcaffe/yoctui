use crate::tests::ux_terminal_fixture;
use crate::*;

fn utility() -> App {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, None);
    app.daemon.pty_details[0].kind = ClientDaemonPtyKind::Utility;
    app.focus = FocusTarget::Workspace;
    app.terminal.browser_open = false;
    app.terminal.retained_output_session = Some(41);
    app
}

#[test]
fn completed_utility_report_stays_read_only_until_returning_to_the_tree() {
    for lifecycle in [
        ClientDaemonLifecycle::Exited,
        ClientDaemonLifecycle::Failed,
        ClientDaemonLifecycle::Cancelled,
    ] {
        let mut app = utility();
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        app.reconcile_terminal_panes();
        assert!(app.terminal_console_fullscreen());
        assert_eq!(app.selected_terminal_screen().unwrap().rows[1], "ready");
        assert!(app.terminal_browser_indexes().is_empty());
        assert!(!app.selected_terminal_is_writer());
        assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
        assert_eq!(
            update(&mut app, Action::TerminalStagePaste("unsafe input".into())),
            None
        );
        assert_eq!(update(&mut app, Action::TerminalConfirmPaste), None);
        update(&mut app, Action::TerminalReleaseControl);
        assert!(app.terminal_browser_visible());
        app.reconcile_terminal_panes();
        assert!(app.terminal_browser_visible());
    }
}

#[test]
fn pending_terminal_binds_new_id_after_old_history_eviction_not_old_vector_slot() {
    let mut app = utility();
    app.prepare_created_terminal_selection();
    app.terminal.retain_created_output = true;
    let mut session = app.daemon.pty_sessions.remove(0);
    let mut details = app.daemon.pty_details.remove(0);
    app.daemon.pty_screens.clear();
    app.reconcile_terminal_panes();
    assert!(app.terminal.awaiting_session);
    assert_eq!(app.selected_terminal_session(), None);
    session.id = 42;
    session.lifecycle = ClientDaemonLifecycle::Exited;
    details.id = 42;
    app.daemon.pty_sessions.push(session);
    app.daemon.pty_details.push(details);
    // The command can finish before its final screen arrives.
    app.reconcile_terminal_panes();
    assert!(!app.terminal.awaiting_session);
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert!(app.terminal_console_fullscreen());
    assert_eq!(app.terminal.pane_sessions[0].1, Some(42));
    app.reconcile_terminal_panes();
    assert!(app.terminal_console_fullscreen());
}

#[test]
fn new_terminal_binding_survives_later_eviction_and_reordered_snapshots() {
    let mut app = utility();
    app.prepare_created_terminal_selection();
    let mut created = app.daemon.pty_sessions[0].clone();
    created.id = 42;
    app.daemon.pty_sessions.push(created);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    app.daemon.pty_sessions.swap(0, 1);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    app.daemon.pty_sessions.retain(|session| session.id == 42);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
}

#[test]
fn utility_report_retention_does_not_hide_loss_removal_or_new_daemon_identity() {
    for lifecycle in [
        ClientDaemonLifecycle::Lost,
        ClientDaemonLifecycle::Disconnected,
    ] {
        let mut app = utility();
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        app.reconcile_terminal_panes();
        assert!(app.terminal_browser_visible());
    }
    let mut app = utility();
    app.daemon.pty_sessions.clear();
    app.reconcile_terminal_panes();
    assert!(app.terminal_browser_visible());
    let mut app = utility();
    app.prepare_created_terminal_selection();
    app.daemon.instance_id = Some(DaemonModelInstanceId([99; 16]));
    let mut foreign = app.daemon.pty_sessions[0].clone();
    foreign.id = 42;
    app.daemon.pty_sessions.push(foreign);
    app.reconcile_terminal_panes();
    assert!(!app.terminal.awaiting_session);
    assert_eq!(app.selected_terminal_session(), None);
    assert!(app.terminal_browser_visible());
}

#[test]
fn pending_terminal_never_selects_an_old_retained_session_after_eviction() {
    let mut app = utility();
    app.prepare_created_terminal_selection();
    app.daemon.pty_sessions[0].id = 12;
    app.reconcile_terminal_panes();
    assert!(app.terminal.awaiting_session);
    assert_eq!(app.selected_terminal_session(), None);
    assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
}

#[test]
fn ordinary_utility_exit_and_another_session_do_not_inherit_debug_report_retention() {
    let mut app = utility();
    app.terminal.retained_output_session = None;
    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Exited;
    app.reconcile_terminal_panes();
    assert!(app.terminal_browser_visible());
    let mut app = utility();
    app.terminal.retained_output_session = Some(999);
    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Exited;
    app.reconcile_terminal_panes();
    assert!(app.terminal_browser_visible());
}
