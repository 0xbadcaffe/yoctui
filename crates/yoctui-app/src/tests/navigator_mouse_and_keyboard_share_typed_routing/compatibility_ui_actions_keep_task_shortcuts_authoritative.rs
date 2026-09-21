use super::*;

#[test]
fn compatibility_ui_actions_keep_task_shortcuts_authoritative() {
    let definitions = yoctui_model::compatibility_ui_workspace_action_definitions(
        yoctui_model::WorkspaceDestination::Tasks,
    );
    let shortcut = |id: &str| {
        definitions
            .iter()
            .find(|action| action.id == id)
            .map(|action| action.shortcut)
    };
    assert_eq!(shortcut("tasks.build"), Some("B"));
    assert_eq!(key_action(Input::Char('B')), Some(Action::OpenBuildOptions));
    assert_eq!(shortcut("tasks.cancel"), Some("c"));
    assert_eq!(key_action(Input::Char('c')), Some(Action::Cancel));
    assert_eq!(shortcut("tasks.logs"), Some("l"));
    assert_eq!(
        key_action(Input::Char('l')),
        Some(Action::Open(Screen::Logs))
    );
    assert_eq!(shortcut("tasks.history"), Some("h"));
    assert_eq!(
        key_action(Input::Char('h')),
        Some(Action::Open(Screen::BuildHistory))
    );
}
