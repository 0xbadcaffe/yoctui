use super::*;

fn fixture() -> App {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    app.daemon.instance_id = Some(DaemonModelInstanceId([9; 16]));
    let mut other = app.daemon.pty_sessions[0].clone();
    other.id = 42;
    other.name = "unrelated".into();
    app.daemon.pty_sessions.push(other);
    app
}

#[test]
fn terminal_kill_safety_focus_and_cancel_require_a_fresh_review() {
    let mut app = fixture();
    app.screen = Screen::Dashboard;
    app.focus = FocusTarget::Navigator;
    assert_eq!(update(&mut app, Action::TerminalBeginKill), None);
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::KillConfirmation);
    assert_eq!(update(&mut app, Action::TerminalCancelMode), None);
    assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
    update(&mut app, Action::TerminalBeginKill);
    assert_eq!(
        update(&mut app, Action::TerminalConfirmKill),
        Some(Effect::Terminal(TerminalEffect::Terminate {
            session_id: 41
        }))
    );
    assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
}

#[test]
fn terminal_kill_safety_changed_selection_cannot_retarget_confirmation() {
    let mut app = fixture();
    update(&mut app, Action::TerminalBeginKill);
    app.select_terminal_session(1);
    assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
}

#[test]
fn terminal_kill_safety_invalid_snapshots_cancel_but_stable_reorder_is_safe() {
    for invalidation in 0..5 {
        let mut app = fixture();
        update(&mut app, Action::TerminalBeginKill);
        match invalidation {
            0 => app.daemon.status = ClientReplicaStatus::Stale,
            1 => app.daemon.status = ClientReplicaStatus::Disconnected,
            2 => app.daemon.instance_id = Some(DaemonModelInstanceId([10; 16])),
            3 => {
                app.daemon.pty_sessions.remove(0);
            }
            _ => app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Exited,
        }
        app.reconcile_terminal_panes();
        assert_eq!(
            app.terminal.mode,
            TerminalWorkbenchMode::Live,
            "{invalidation}"
        );
        assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
    }
    let mut app = fixture();
    update(&mut app, Action::TerminalBeginKill);
    app.daemon.pty_sessions.swap(0, 1);
    app.reconcile_terminal_panes();
    assert_eq!(
        update(&mut app, Action::TerminalConfirmKill),
        Some(Effect::Terminal(TerminalEffect::Terminate {
            session_id: 41
        }))
    );
}

#[test]
fn terminal_kill_safety_confirmation_rechecks_authority_without_reconciliation() {
    for invalidation in 0..4 {
        let mut app = fixture();
        update(&mut app, Action::TerminalBeginKill);
        match invalidation {
            0 => app.daemon.status = ClientReplicaStatus::Synchronizing,
            1 => app.daemon.instance_id = Some(DaemonModelInstanceId([11; 16])),
            2 => app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Stopping,
            _ => {
                app.daemon.pty_sessions.clear();
            }
        }
        assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
        assert!(app.terminal.kill_target.is_none());
    }
}

#[test]
fn terminal_kill_safety_pane_changes_invalidate_and_paste_cannot_replace_review() {
    let mut app = fixture();
    update(&mut app, Action::TerminalBeginKill);
    let target = app.terminal.kill_target.clone();
    assert_eq!(
        update(&mut app, Action::TerminalStagePaste("exit\n".into())),
        None
    );
    assert_eq!(app.terminal.kill_target, target);
    assert!(app.terminal.pending_paste.is_empty());
    app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
    assert_eq!(update(&mut app, Action::TerminalConfirmKill), None);
    update(&mut app, Action::TerminalBeginKill);
    app.close_terminal_pane().unwrap();
    assert_eq!(app.terminal.mode, TerminalWorkbenchMode::Live);
    assert!(app.terminal.kill_target.is_none());
}

#[test]
fn terminal_kill_safety_ended_history_close_and_unavailable_begin_are_unchanged() {
    let mut app = fixture();
    app.daemon.pty_sessions[0].lifecycle = ClientDaemonLifecycle::Exited;
    assert_eq!(
        update(&mut app, Action::TerminalBeginKill),
        Some(Effect::Terminal(TerminalEffect::Close { session_id: 41 }))
    );
    assert!(app.terminal.kill_target.is_none());
    app.daemon.status = ClientReplicaStatus::Disconnected;
    assert_eq!(update(&mut app, Action::TerminalBeginKill), None);
    assert!(app.terminal.kill_target.is_none());
}
