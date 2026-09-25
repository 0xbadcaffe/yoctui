use super::*;

#[test]
fn devtool_undeploy_and_upgrade_use_distinct_validated_confirmations() {
    let mut app = App::new(10, 1_000);
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/busybox.bb"),
    };
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
                source_path: PathBuf::from("/workspace/busybox"),
                recipe_file: Some(identity.file.clone()),
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );

    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUndeploy);
    for character in "root@board".chars() {
        let _ = update(&mut app, Action::AppendDevtoolUndeployTarget(character));
    }
    let _ = update(&mut app, Action::PreviewDevtoolUndeploy);
    let undeploy = DevtoolUndeployPlan {
        identity: identity.clone(),
        target: "root@board".into(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolUndeployConfirmation(undeploy.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolUndeploy),
        Some(Effect::DevtoolUndeploy(undeploy))
    );

    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolUpgrade);
    let upgrade = DevtoolUpgradePlan { identity };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolUpgradeConfirmation(upgrade.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolUpgrade),
        Some(Effect::DevtoolUpgrade(upgrade))
    );
}
