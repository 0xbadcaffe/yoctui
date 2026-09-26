use super::*;

fn patch_app() -> (App, RecipeIdentity, Layer) {
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta-core/recipes-core/busybox/busybox.bb".into(),
    };
    let layer = Layer {
        name: "meta-custom".into(),
        path: "/layers/meta-custom".into(),
        priority: Some(8),
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    app.workspace.layers = vec![layer.clone()];
    app.workspace.recipes = vec![Recipe {
        name: identity.name.clone(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: "/build/workspace/sources/busybox".into(),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::Available {
                repository_root: Some("/build/workspace/sources/busybox".into()),
                branch: Some("devtool".into()),
                upstream: None,
                ahead: 0,
                behind: 0,
                head: Some("abc123".into()),
                modified: 1,
                untracked: 0,
                conflicted: 0,
            },
            error: None,
        },
    );
    (app, identity, layer)
}

#[test]
fn devtool_patch_picker_retains_recipe_and_configured_layer_until_confirmation() {
    let (mut app, identity, layer) = patch_app();
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolPatch);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::DevtoolPatchPicker(picker))
            if picker.identity == identity && picker.layers == vec![layer.clone()]
    ));

    let _ = update(&mut app, Action::PreviewDevtoolPatch);
    let plan = DevtoolPatchPlan { identity, layer };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolPatchConfirmation(plan.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolPatch),
        Some(Effect::DevtoolUpdateRecipePatch(plan))
    );
}

#[test]
fn devtool_patch_confirmation_rejects_a_layer_removed_after_preview() {
    let (mut app, _, _) = patch_app();
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolPatch);
    let _ = update(&mut app, Action::PreviewDevtoolPatch);
    app.workspace.layers.clear();

    assert_eq!(update(&mut app, Action::ConfirmDevtoolPatch), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("The selected patch layer is no longer configured.")
    );
}
