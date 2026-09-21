use super::*;

#[test]
fn raw_navigation_routes_workspace_and_shared_focus_input() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::RawMode;
    assert_eq!(
        workspace_collection_action(&app, Input::Down),
        Some(Action::RawMode(
            yoctui_model::RawModeAction::SelectCategory { delta: 1 }
        ))
    );
    assert_eq!(
        raw_mode_input(&app.raw_mode, Input::Char('/')),
        Some(yoctui_model::RawModeAction::BeginSearch)
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::Tab),
        Some(Action::CycleFocus { backwards: false })
    );
    assert_eq!(
        focus_action(FocusTarget::Workspace, Input::BackTab),
        Some(Action::CycleFocus { backwards: true })
    );
}
