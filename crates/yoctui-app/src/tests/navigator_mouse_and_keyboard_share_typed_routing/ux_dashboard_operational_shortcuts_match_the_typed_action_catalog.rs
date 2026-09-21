use super::*;

#[test]
fn ux_dashboard_operational_shortcuts_match_the_typed_action_catalog() {
    let definitions = yoctui_model::compatibility_ui_workspace_action_definitions(
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    let shortcut = |id: &str| {
        definitions
            .iter()
            .find(|action| action.id == id)
            .map(|action| action.shortcut)
    };
    for (id, expected, input, action) in [
        (
            "dashboard.errors",
            "e",
            Input::Char('e'),
            Action::Open(Screen::Errors),
        ),
        (
            "dashboard.environment",
            "E",
            Input::Char('E'),
            Action::Open(Screen::BuildEnvironment),
        ),
        (
            "dashboard.maintenance",
            "M",
            Input::Char('M'),
            Action::Open(Screen::Maintenance),
        ),
    ] {
        assert_eq!(shortcut(id), Some(expected));
        assert_eq!(key_action(input), Some(action));
    }
    assert_eq!(shortcut("dashboard.tasks"), Some("F2"));
    assert_eq!(key_action(Input::F2), Some(Action::Open(Screen::Tasks)));
    assert_eq!(shortcut("dashboard.history"), Some("F3"));
    assert_eq!(
        key_action(Input::F3),
        Some(Action::Open(Screen::BuildHistory))
    );
    assert_eq!(shortcut("dashboard.artifacts"), Some("F8"));
    assert_eq!(key_action(Input::F8), Some(Action::Open(Screen::Images)));
}
