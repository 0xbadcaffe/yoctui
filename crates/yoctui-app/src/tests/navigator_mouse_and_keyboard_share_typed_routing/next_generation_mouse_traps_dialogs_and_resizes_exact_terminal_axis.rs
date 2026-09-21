use super::*;

#[test]
fn next_generation_mouse_traps_dialogs_and_resizes_exact_terminal_axis() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    let second = app
        .pane_layout
        .split(yoctui_model::PaneId(1), SplitAxis::Horizontal)
        .unwrap();
    for id in 1..=2 {
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: format!("shell-{id}"),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
    }
    let select_first = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 40,
            row: 10,
        },
        &app,
        120,
        30,
    );
    assert_eq!(
        select_first,
        Some(Action::SelectPtyPane {
            pane: yoctui_model::PaneId(1),
            index: 0,
        })
    );
    let _ = yoctui_model::update(&mut app, select_first.unwrap());
    assert_eq!(app.pane_layout.focused, yoctui_model::PaneId(1));

    let select_second = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 90,
            row: 10,
        },
        &app,
        120,
        30,
    );
    assert_eq!(
        select_second,
        Some(Action::SelectPtyPane {
            pane: second,
            index: 1,
        })
    );
    let _ = yoctui_model::update(&mut app, select_second.unwrap());
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 90,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        Some(Action::ResizeFocusedPane {
            delta_per_mille: 193,
        })
    );

    app.dialogs.push_back(yoctui_model::Dialog::ThemePicker {
        selection: 0,
        original_theme: app.theme,
        original_color_enabled: app.color_enabled,
        original_settings_dirty: app.settings_dirty,
    });
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ScrollDown,
                column: 90,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        Some(Action::SelectTheme { delta: 1 })
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 10,
                row: 10,
            },
            &app,
            120,
            30,
        ),
        None,
        "a modal traps drag input instead of leaking to terminal resizing"
    );
}
