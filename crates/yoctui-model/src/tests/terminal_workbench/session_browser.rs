use crate::tests::ux_terminal_fixture;
use crate::*;

#[test]
fn terminal_browser_open_release_and_reopen_preserve_running_process() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    update(&mut app, Action::Open(Screen::TerminalSessions));
    assert!(app.terminal_browser_visible());
    assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
    assert!(app.terminal_console_fullscreen());
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(matches!(
        update(&mut app, Action::TerminalReleaseControl),
        Some(Effect::Terminal(TerminalEffect::ReleaseControl {
            session_id: 41,
            writer_epoch: 9
        }))
    ));
    assert!(app.terminal_browser_visible());
    assert_eq!(
        app.daemon.pty_sessions[0].lifecycle,
        ClientDaemonLifecycle::Running
    );
    assert_eq!(app.terminal_browser_indexes(), vec![0]);
    app.daemon.pty_details[0].writer = None;
    assert!(matches!(
        update(&mut app, Action::TerminalTakeControl),
        Some(Effect::Terminal(TerminalEffect::TakeControl {
            session_id: 41,
            expected_epoch: 9
        }))
    ));
    assert!(app.terminal_console_fullscreen());
}

#[test]
fn terminal_browser_immediate_reopen_waits_for_release_and_uses_new_epoch_once() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    update(&mut app, Action::Open(Screen::TerminalSessions));
    update(&mut app, Action::TerminalTakeControl);
    update(&mut app, Action::TerminalReleaseControl);
    assert!(
        !app.selected_terminal_is_writer(),
        "released input must stop immediately"
    );
    assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
    assert_eq!(app.pending_terminal_writer_effect(), None);
    app.daemon.pty_details[0].writer = None;
    app.daemon.pty_details[0].writer_epoch = 10;
    assert_eq!(
        app.pending_terminal_writer_effect(),
        Some(TerminalEffect::TakeControl {
            session_id: 41,
            expected_epoch: 10,
        })
    );
    assert_eq!(
        app.pending_terminal_writer_effect(),
        None,
        "no duplicate request"
    );
    app.daemon.pty_details[0].writer = Some([7; 16]);
    app.daemon.pty_details[0].writer_epoch = 11;
    assert!(app.selected_terminal_is_writer());
}

#[test]
fn terminal_browser_deferred_reopen_never_takes_a_foreign_writer_or_new_daemon() {
    for replace_daemon in [false, true] {
        let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
        update(&mut app, Action::Open(Screen::TerminalSessions));
        update(&mut app, Action::TerminalTakeControl);
        update(&mut app, Action::TerminalReleaseControl);
        update(&mut app, Action::TerminalTakeControl);
        app.daemon.pty_details[0].writer = Some([8; 16]);
        app.daemon.pty_details[0].writer_epoch = 10;
        if replace_daemon {
            app.daemon.instance_id = Some(DaemonModelInstanceId([99; 16]));
            app.reconcile_terminal_panes();
        }
        assert_eq!(app.pending_terminal_writer_effect(), None);
        assert_eq!(app.terminal.releasing_writer, None);
        assert!(!app.terminal.reopen_after_release);
        assert!(!app.selected_terminal_is_writer());
    }
}

#[test]
fn terminal_browser_filters_unavailable_history_and_scrolls_to_final_live_session() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Exited, None);
    for id in 42..=142 {
        let mut session = app.daemon.pty_sessions[0].clone();
        session.id = id;
        session.lifecycle = if id % 2 == 0 {
            ClientDaemonLifecycle::Running
        } else {
            ClientDaemonLifecycle::Lost
        };
        app.daemon.pty_sessions.push(session);
    }
    update(&mut app, Action::Open(Screen::TerminalSessions));
    update(&mut app, Action::SelectPtySession { delta: isize::MAX });
    assert_eq!(app.selected_terminal_session().unwrap().id, 142);
    update(&mut app, Action::SelectPtySession { delta: isize::MIN });
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert_eq!(app.daemon.pty_sessions.len(), 102, "history is not deleted");
    app.daemon.pty_sessions[1].lifecycle = ClientDaemonLifecycle::Exited;
    app.terminal.browser_open = false;
    app.reconcile_terminal_panes();
    assert!(app.terminal_browser_visible());
    assert!(!app.terminal_browser_indexes().contains(&1));
}

#[test]
fn terminal_browser_remote_writer_is_viewed_without_stealing_lease() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([8; 16]));
    update(&mut app, Action::Open(Screen::TerminalSessions));
    assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
    assert!(app.terminal_console_fullscreen());
    assert!(!app.selected_terminal_is_writer());
    assert_eq!(update(&mut app, Action::TerminalReleaseControl), None);
    assert!(app.terminal_browser_visible());
    assert_eq!(app.daemon.pty_details[0].writer, Some([8; 16]));
}

#[test]
fn terminal_browser_does_not_rebind_pending_launch_or_send_paste_from_list() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    app.prepare_created_terminal_selection();
    app.reconcile_terminal_panes();
    assert!(app.terminal.awaiting_session);
    assert!(!app.terminal.browser_open);
    assert_eq!(app.selected_terminal_session(), None);
    let mut session = app.daemon.pty_sessions[0].clone();
    session.id = 42;
    app.daemon.pty_sessions.push(session);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert!(!app.terminal.awaiting_session);
    app.select_terminal_session(-1);
    app.terminal.browser_open = true;
    update(
        &mut app,
        Action::TerminalStagePaste("echo forbidden".into()),
    );
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
    assert_eq!(update(&mut app, Action::TerminalConfirmPaste), None);
}
