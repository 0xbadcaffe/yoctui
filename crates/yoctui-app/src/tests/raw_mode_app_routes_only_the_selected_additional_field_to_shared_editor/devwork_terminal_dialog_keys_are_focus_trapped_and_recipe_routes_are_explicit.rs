use super::*;

#[test]
fn devwork_terminal_dialog_keys_are_focus_trapped_and_recipe_routes_are_explicit() {
    assert_eq!(
        terminal_launch_dialog_action(Input::Down),
        Some(Action::SelectTerminalLaunchDestination { delta: 1 })
    );
    assert_eq!(
        terminal_launch_dialog_action(Input::Enter),
        Some(Action::ConfirmTerminalLaunch)
    );
    assert_eq!(
        terminal_launch_dialog_action(Input::Esc),
        Some(Action::CancelTerminalLaunch)
    );
    assert_eq!(terminal_launch_dialog_action(Input::Char('x')), None);
    assert_eq!(
        recipes_workspace_action(false, Input::Char('s')),
        Some(Action::BeginSelectedRecipeDevtoolWorkspaceShell)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('E')),
        Some(Action::BeginSelectedRecipeDevtoolEditRecipe)
    );
}
