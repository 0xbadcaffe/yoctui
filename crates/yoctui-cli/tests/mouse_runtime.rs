use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
use yoctui_model::{
    Action, App, ClientDaemonLifecycle, ClientDaemonPtySummary, Dialog, FocusTarget,
};

#[test]
fn mouse_runtime_routes_dashboard_terminal_and_dialog_focus() {
    let mut app = App::new(16, 4096);
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 1,
        name: "shell".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 2,
            },
            &app,
            120,
        ),
        Some(Action::SelectPtySession { delta: 1 })
    );
    app.dialogs.push_back(Dialog::BuildOptions);
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 2,
            },
            &app,
            120,
        ),
        Some(Action::Focus(FocusTarget::Dialog))
    );
}
