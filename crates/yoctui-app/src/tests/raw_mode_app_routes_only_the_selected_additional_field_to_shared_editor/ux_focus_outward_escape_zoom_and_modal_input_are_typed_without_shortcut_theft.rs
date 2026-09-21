use super::*;

#[test]
fn ux_focus_outward_escape_zoom_and_modal_input_are_typed_without_shortcut_theft() {
    let mut app = yoctui_model::App::new(16, 4_096);
    app.screen = Screen::Tasks;
    app.focus = FocusTarget::Workspace;
    let _ = yoctui_model::update(&mut app, Action::CyclePaneSubfocus { backwards: false });
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::ResetPaneSubfocus)
    );
    let _ = yoctui_model::update(&mut app, Action::ResetPaneSubfocus);
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    assert_eq!(focus_action_for_app(&app, Input::Char('z')), None);

    let _ = yoctui_model::update(&mut app, Action::TogglePaneZoom);
    assert_eq!(
        focus_action_for_app(&app, Input::Esc),
        Some(Action::TogglePaneZoom)
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 2,
                row: 8,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Focus(FocusTarget::Workspace)),
        "a zoomed pane owns its whole body instead of leaking clicks"
    );

    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(focus_action_for_app(&app, Input::Tab), None);
    assert_eq!(focus_action_for_app(&app, Input::Esc), None);
    assert_eq!(
        menu_action(&app, Input::Esc),
        Some(MenuInputResult::Reduce(Box::new(Action::CloseMenu)))
    );
}
