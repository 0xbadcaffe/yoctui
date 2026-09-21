use super::*;

#[test]
fn devtool_metadata_shortcut_requests_typed_status() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
}
