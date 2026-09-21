use super::*;

#[test]
fn recipe_navigation_maps_files_logs_patches_and_devtool_routes() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('e')),
        Some(Action::OpenSelectedRecipeProvider)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('o')),
        Some(Action::BeginSelectedRecipeTaskLog)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('p')),
        Some(Action::BeginSelectedRecipePatchReview)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('d')),
        Some(Action::BeginSelectedRecipeDevtoolModify)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSelectedRecipeDevtoolStatus)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('u')),
        Some(Action::BeginSelectedRecipeDevtoolUpdateRecipe)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('F')),
        Some(Action::BeginSelectedRecipeDevtoolFinish)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('P')),
        Some(Action::BeginSelectedRecipeDevtoolDeploy)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('D')),
        Some(Action::BeginSelectedRecipeDevtoolReset)
    );
}
