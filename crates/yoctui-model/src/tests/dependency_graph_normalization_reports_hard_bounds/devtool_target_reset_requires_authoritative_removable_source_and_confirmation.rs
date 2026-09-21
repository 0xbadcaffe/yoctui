use super::*;

#[test]
fn devtool_target_reset_requires_authoritative_removable_source_and_confirmation() {
    let mut app = App::new(10, 1_000);
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    let source_path = PathBuf::from("/build/workspace/sources/busybox");
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before reset.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: source_path.clone(),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolReset);
    let plan = DevtoolResetPlan {
        identity: identity.clone(),
        source_path: source_path.clone(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolResetConfirmation(plan.clone()))
    );
    app.devtool_statuses.get_mut(&identity).unwrap().workspace =
        DevtoolWorkspace::MissingDirectory {
            source_path: PathBuf::from("/build/workspace/sources/moved"),
        };
    assert_eq!(update(&mut app, Action::ConfirmDevtoolReset), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("The authoritative Devtool reset source changed; refresh with t.")
    );
    app.devtool_statuses.get_mut(&identity).unwrap().workspace =
        DevtoolWorkspace::MissingDirectory { source_path };
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolReset),
        Some(Effect::DevtoolReset(plan))
    );
}
