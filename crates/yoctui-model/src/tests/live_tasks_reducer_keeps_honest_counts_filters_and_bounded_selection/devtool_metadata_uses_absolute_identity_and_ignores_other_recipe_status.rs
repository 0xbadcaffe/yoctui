use super::*;

#[test]
fn devtool_metadata_uses_absolute_identity_and_ignores_other_recipe_status() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "busybox".into(),
            file: Some("/layers/core/recipes-core/busybox/busybox_1.0.bb".into()),
            ..Recipe::default()
        },
        Recipe {
            name: "bash".into(),
            file: Some("/layers/core/recipes-extended/bash/bash_5.0.bb".into()),
            ..Recipe::default()
        },
    ];
    let busybox = match update(&mut app, Action::BeginSelectedRecipeDevtoolStatus) {
        Some(Effect::InspectDevtoolStatus(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert!(app.devtool_status_loading.contains(&busybox));

    let bash = RecipeIdentity {
        name: "bash".into(),
        file: "/layers/core/recipes-extended/bash/bash_5.0.bb".into(),
    };
    let _ = update(
        &mut app,
        Action::DevtoolStatusLoaded(DevtoolStatus {
            identity: bash.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        }),
    );
    assert!(
        app.devtool_status_loading.contains(&busybox),
        "a response for another absolute recipe identity is stale for the selection"
    );
    assert_eq!(app.devtool_statuses[&bash].identity, bash);
    assert!(
        app.devtool_statuses[&bash]
            .disabled_reason(DevtoolAction::ModifyOrEdit)
            .is_none()
    );
    assert_eq!(
        app.devtool_statuses[&bash]
            .disabled_reason(DevtoolAction::UpdateRecipe)
            .as_deref(),
        Some("Recipe is not in the Devtool workspace.")
    );
}
