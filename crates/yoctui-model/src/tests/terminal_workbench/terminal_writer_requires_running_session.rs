use crate::tests::ux_terminal_fixture;
use crate::{
    Action, ClientDaemonLifecycle, ClientReplicaStatus, Effect, TerminalWorkbenchMode, update,
};

#[test]
fn terminal_writer_requires_current_running_session_and_matching_identity() {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    assert!(app.selected_terminal_is_writer());
    for lifecycle in [
        ClientDaemonLifecycle::Disconnected,
        ClientDaemonLifecycle::Connecting,
        ClientDaemonLifecycle::Stopping,
        ClientDaemonLifecycle::Exited,
        ClientDaemonLifecycle::Failed,
        ClientDaemonLifecycle::Lost,
    ] {
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        assert!(!app.selected_terminal_is_writer(), "{lifecycle:?}");
        assert_eq!(app.daemon.pty_details[0].writer, Some([7; 16]));
        assert_eq!(app.selected_terminal_screen().unwrap().rows[1], "ready");
    }
    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Running;
    for status in [
        ClientReplicaStatus::Disconnected,
        ClientReplicaStatus::Synchronizing,
        ClientReplicaStatus::Stale,
    ] {
        app.daemon.status = status;
        assert!(!app.selected_terminal_is_writer(), "{status:?}");
    }
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.pty_details[0].writer = Some([8; 16]);
    assert!(!app.selected_terminal_is_writer());
    app.daemon.pty_details[0].writer = None;
    assert!(!app.selected_terminal_is_writer());
    app.terminal.client_id = None;
    assert!(!app.selected_terminal_is_writer());
    app.daemon.pty_sessions.clear();
    assert!(!app.selected_terminal_is_writer());
}

#[test]
fn terminal_exit_between_paste_review_and_confirmation_cannot_emit_input() {
    for lifecycle in [
        ClientDaemonLifecycle::Exited,
        ClientDaemonLifecycle::Failed,
        ClientDaemonLifecycle::Lost,
        ClientDaemonLifecycle::Stopping,
    ] {
        let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
        assert_eq!(
            update(&mut app, Action::TerminalStagePaste("echo safe".into())),
            None
        );
        assert_eq!(app.terminal.mode, TerminalWorkbenchMode::PasteReview);
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        assert_eq!(update(&mut app, Action::TerminalConfirmPaste), None);
        assert_eq!(update(&mut app, Action::TerminalReleaseControl), None);
        assert_eq!(update(&mut app, Action::TerminalTakeControl), None);
        assert_eq!(update(&mut app, Action::TerminalCancelMode), None);
        assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
        assert_eq!(
            update(&mut app, Action::TerminalStagePaste("echo late".into())),
            None
        );
        assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
        assert_eq!(update(&mut app, Action::TerminalEnterCopyMode), None);
        assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Copy);
        assert!(matches!(
            update(&mut app, Action::TerminalCopyViewport),
            Some(Effect::CopyToClipboard(_))
        ));
        assert_eq!(app.daemon.pty_details[0].writer_epoch, 9);
    }
}
