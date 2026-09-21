use super::*;

#[test]
fn devtool_modify_requires_authoritative_status_and_confirmation() {
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
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
        None
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before modifying.")
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity: identity.clone(),
            capability: DevtoolCapability::MissingExecutable,
            workspace: DevtoolWorkspace::NotMember,
            git: DevtoolGitState::NotApplicable,
            error: None,
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
    assert_eq!(
        app.notification.as_deref(),
        Some("Devtool executable is missing.")
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolModify);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolModifyConfirmation(identity.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolModify),
        Some(Effect::DevtoolModify(identity.clone()))
    );
    app.devtool_statuses.insert(
        identity.clone(),
        DevtoolStatus {
            identity,
            capability: DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: PathBuf::from("/build/workspace/sources/busybox"),
                recipe_file: None,
            },
            git: DevtoolGitState::NotRepository,
            error: None,
        },
    );
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDevtoolModify),
        Some(Effect::OpenWorkspaceEditor {
            label: "busybox".into(),
            root: PathBuf::from("/build/workspace/sources/busybox"),
        })
    );
}
