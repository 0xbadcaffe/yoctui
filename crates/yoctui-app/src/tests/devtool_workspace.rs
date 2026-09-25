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
        Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe)
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
