use crate::tests::ux_terminal_fixture;
use crate::{
    Action, App, ClientDaemonLifecycle, ClientReplicaStatus, DaemonModelInstanceId, PaneId,
    PaneLayoutError, SplitAxis, update,
};

fn fixture() -> App {
    let mut app = ux_terminal_fixture(ClientDaemonLifecycle::Running, Some([7; 16]));
    app.daemon.instance_id = Some(DaemonModelInstanceId([1; 16]));
    for id in [42, 43] {
        let mut session = app.daemon.pty_sessions[0].clone();
        session.id = id;
        app.daemon.pty_sessions.push(session);
        let mut details = app.daemon.pty_details[0].clone();
        details.id = id;
        app.daemon.pty_details.push(details);
    }
    app.pty_selection = 2;
    app
}

#[test]
fn terminal_pane_split_selection_focus_close_preserve_other_session_identity() {
    let mut app = fixture();
    let first = app.pane_layout.focused;
    let second = app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(2));
    assert_eq!(
        update(&mut app, Action::SelectPtySession { delta: -1 }),
        None
    );
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(2));
    assert_eq!(
        update(
            &mut app,
            Action::SelectPtyPane {
                pane: first,
                index: 2
            }
        ),
        None
    );
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
    assert_eq!(app.terminal_pane_session_index(second, 1, 2), Some(1));
    assert_eq!(app.close_terminal_pane(), Ok(second));
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert_eq!(app.daemon.pty_sessions.len(), 3);
    assert_eq!(app.terminal.pane_sessions.len(), 1);
    let before = (app.pane_layout.clone(), app.terminal.clone());
    assert_eq!(app.close_terminal_pane(), Err(PaneLayoutError::LastPane));
    assert_eq!((app.pane_layout.clone(), app.terminal.clone()), before);
}

#[test]
fn terminal_pane_bindings_survive_reordering_but_never_rebind_removed_ids() {
    let mut app = fixture();
    let first = app.pane_layout.focused;
    app.split_terminal_pane(SplitAxis::Vertical).unwrap();
    update(&mut app, Action::SelectPtySession { delta: -1 });
    app.daemon.pty_sessions.remove(0);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 42);
    assert_eq!(app.pty_selection, 0);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(1));
    let removed = app.daemon.pty_sessions.remove(0);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session(), None);
    assert!(!app.selected_terminal_is_writer());
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(0));
    app.daemon.pty_sessions.push(removed);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session(), None);
    assert!(app.select_terminal_pane(first, 0));
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
}

#[test]
fn terminal_pane_replaced_daemon_cannot_reuse_bound_session_authority() {
    let mut app = fixture();
    let first = app.pane_layout.focused;
    app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
    app.daemon.status = ClientReplicaStatus::Stale;
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
    assert!(!app.selected_terminal_is_writer());
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.instance_id = Some(DaemonModelInstanceId([2; 16]));
    assert_eq!(app.selected_terminal_session(), None);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), None);
    app.reconcile_terminal_panes();
    assert!(
        app.terminal
            .pane_sessions
            .iter()
            .all(|(_, id)| id.is_none())
    );
    assert_eq!(app.selected_terminal_session(), None);
    assert!(!app.selected_terminal_is_writer());
    assert!(app.select_terminal_pane(first, 2));
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
}

#[test]
fn terminal_pane_invalid_focus_and_empty_selection_are_bounded() {
    let mut app = fixture();
    let before = (
        app.pane_layout.clone(),
        app.terminal.clone(),
        app.pty_selection,
    );
    assert!(!app.select_terminal_pane(PaneId(99), 0));
    assert!(!app.select_terminal_pane(app.pane_layout.focused, usize::MAX));
    assert_eq!(
        (
            app.pane_layout.clone(),
            app.terminal.clone(),
            app.pty_selection
        ),
        before
    );
    update(&mut app, Action::SelectPtySession { delta: isize::MAX });
    assert_eq!(app.selected_terminal_session().unwrap().id, 43);
    update(&mut app, Action::SelectPtySession { delta: isize::MIN });
    assert_eq!(app.selected_terminal_session().unwrap().id, 41);
    app.daemon.pty_sessions.clear();
    update(&mut app, Action::SelectPtySession { delta: 1 });
    assert_eq!(app.selected_terminal_session(), None);
    assert_eq!(app.terminal.pane_sessions.len(), 1);
}

#[test]
fn terminal_confirmed_launch_selection_is_not_shadowed_by_an_old_pane_binding() {
    let mut app = fixture();
    let first = app.pane_layout.focused;
    app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
    update(&mut app, Action::SelectPtySession { delta: -1 });
    app.prepare_created_terminal_selection();
    assert_eq!(app.selected_terminal_session(), None);
    let mut created = app.daemon.pty_sessions[0].clone();
    created.id = 44;
    app.daemon.pty_sessions.push(created);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 44);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(2));
}
