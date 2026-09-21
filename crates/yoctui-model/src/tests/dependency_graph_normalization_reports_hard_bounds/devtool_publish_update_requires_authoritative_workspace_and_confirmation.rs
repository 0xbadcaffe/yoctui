use super::*;

#[test]
fn devtool_publish_update_requires_authoritative_workspace_and_confirmation() {
    let mut app = App::new(10, 1_000);
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before update-recipe.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.notification.as_deref(),
        Some("Recipe is not in the Devtool workspace.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpdateRecipe);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolUpdateConfirmation(identity.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolUpdateRecipe),
        Some(Effect::DevtoolUpdateRecipe(identity))
    );
}
