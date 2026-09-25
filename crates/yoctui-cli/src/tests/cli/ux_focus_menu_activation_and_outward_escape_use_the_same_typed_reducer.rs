use super::*;

#[test]
fn ux_focus_menu_activation_and_outward_escape_use_the_same_typed_reducer() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Tasks;
    app.focus = yoctui_model::FocusTarget::Workspace;
    let _ = update(&mut app, Action::CyclePaneSubfocus { backwards: false });
    assert_eq!(
        pane_focus_route(&app, Input::Esc),
        Some(Action::ResetPaneSubfocus)
    );
    let _ = update(&mut app, Action::ResetPaneSubfocus);

    let _ = update(&mut app, Action::OpenApplicationMenu);
    let _ = update(&mut app, Action::SelectMenuGroup { delta: 4 });
    let inspector_index = app
        .active_menu_items()
        .iter()
        .position(|item| item.action_id.as_str() == "view.focus-inspector")
        .unwrap();
    let _ = update(
        &mut app,
        Action::SelectMenuItem {
            delta: inspector_index as isize,
        },
    );
    assert_eq!(
        app.selected_menu_item()
            .and_then(|item| item.disabled_reason),
        Some("The Inspector is read-only".into())
    );
    assert_eq!(
        menu_action(&app, Input::Enter),
        Some(MenuInputResult::ActivateDisabled(
            "The Inspector is read-only".into()
        ))
    );

    let workspace_index = app
        .active_menu_items()
        .iter()
        .position(|item| item.action_id.as_str() == "view.focus-workspace")
        .unwrap();
    let _ = update(
        &mut app,
        Action::SelectMenuItem {
            delta: workspace_index as isize - inspector_index as isize,
        },
    );
    let Some(MenuInputResult::ActivateCommand(command)) = menu_action(&app, Input::Enter) else {
        panic!("actionable focus command must activate from the typed View menu")
    };
    let _ = update(&mut app, Action::CloseMenu);
    let action = yoctui_model::command_action(&app, command);
    let _ = update(&mut app, action);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Workspace);

    let _ = update(
        &mut app,
        Action::Focus(yoctui_model::FocusTarget::Workspace),
    );
    assert_eq!(app.focus, yoctui_model::FocusTarget::Workspace);

    let _ = update(&mut app, Action::TogglePaneZoom);
    assert_eq!(app.zoomed_pane, Some(yoctui_model::FocusTarget::Workspace));
    assert_eq!(
        pane_focus_route(&app, Input::Esc),
        Some(Action::TogglePaneZoom)
    );
}
