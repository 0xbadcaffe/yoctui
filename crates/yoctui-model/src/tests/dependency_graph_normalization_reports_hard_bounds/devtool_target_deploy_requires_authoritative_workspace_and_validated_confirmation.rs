use super::*;

#[test]
fn devtool_target_deploy_requires_authoritative_workspace_and_validated_confirmation() {
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before deploy-target.")
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolDeploy);
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('q'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('e'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('m'));
    let _ = update(&mut app, Action::AppendDevtoolDeployTarget('u'));
    let _ = update(&mut app, Action::PreviewDevtoolDeploy);
    let plan = DevtoolDeployPlan {
        identity: identity.clone(),
        target: "qemu".into(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolDeployConfirmation(plan.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolDeploy),
        Some(Effect::DevtoolDeploy(plan))
    );

    app.dialogs
        .push_back(Dialog::DevtoolDeploy(DevtoolDeployDraft {
            identity,
            target: "--help".into(),
        }));
    assert_eq!(update(&mut app, Action::PreviewDevtoolDeploy), None);
    assert_eq!(
        app.notification.as_deref(),
        Some(
            "Devtool target must be one non-option value without whitespace or control characters"
        )
    );
}
