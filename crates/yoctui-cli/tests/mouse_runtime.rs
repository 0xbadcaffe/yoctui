use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
use yoctui_model::{
    Action, App, ClientDaemonLifecycle, ClientDaemonPtySummary, Dialog, FocusTarget, PaneId,
    SplitAxis,
};

#[test]
fn next_generation_mouse_runtime_routes_exact_terminal_and_dialog_focus() {
    let mut app = App::new(16, 4096);
    let second = app
        .pane_layout
        .split(PaneId(1), SplitAxis::Horizontal)
        .unwrap();
    for id in 1..=2 {
        app.daemon.pty_sessions.push(ClientDaemonPtySummary {
            id,
            name: format!("shell-{id}"),
            lifecycle: ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    }
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 90,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        Some(Action::SelectPtyPane {
            pane: second,
            index: 1,
        })
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
            30,
        ),
        Some(Action::Focus(FocusTarget::Dialog))
    );
}
