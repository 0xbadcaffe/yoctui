use super::*;

#[test]
fn devtool_workspace_routes_the_ordered_recipe_development_loop() {
    assert_eq!(
        devtool_workspace_action(false, Input::Enter),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('d')),
        Some(Action::BeginSelectedRecipeDevtoolModify)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('b')),
        Some(Action::BeginSelectedRecipeBuild)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('P')),
        Some(Action::BeginSelectedRecipeDevtoolDeploy)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('u')),
        Some(Action::BeginSelectedRecipeDevtoolPatch)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('F')),
        Some(Action::BeginSelectedRecipeDevtoolFinish)
    );
    assert_eq!(
        devtool_workspace_action(false, Input::Char('G')),
        Some(Action::BeginSelectedRecipeDevtoolGitUi)
    );
}

#[test]
fn devtool_patch_dialogs_route_only_bounded_selection_preview_and_confirmation() {
    assert_eq!(
        devtool_patch_picker_action(Input::Up),
        Some(Action::SelectDevtoolPatchLayer { delta: -1 })
    );
    assert_eq!(
        devtool_patch_picker_action(Input::Enter),
        Some(Action::PreviewDevtoolPatch)
    );
    assert_eq!(
        devtool_patch_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolPatch)
    );
    assert_eq!(
        devtool_patch_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolPatchConfirmation)
    );
}

#[test]
fn devtool_workspace_search_keeps_recipe_selection_actions_bounded() {
    assert_eq!(
        devtool_workspace_action(true, Input::Char('b')),
        Some(Action::AppendMetadataQuery('b'))
    );
    assert_eq!(
        devtool_workspace_action(false, Input::PageDown),
        Some(Action::SelectRecipe { delta: 10 })
    );
}
