use super::*;

#[test]
fn devtool_publish_finish_requires_clean_status_and_configured_layer_confirmation() {
    let mut app = App::new(10, 1_000);
    let destination = Layer {
        name: "meta-demo".into(),
        path: PathBuf::from("/layers/meta-demo"),
        priority: Some(7),
    };
    app.workspace.layers = vec![
        Layer {
            name: "meta-core".into(),
            path: PathBuf::from("/layers/meta-core"),
            priority: Some(5),
        },
        destination.clone(),
        Layer {
            name: "relative".into(),
            path: PathBuf::from("layers/relative"),
            priority: None,
        },
    ];
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta-core/recipes-core/busybox/busybox.bb"),
    };
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        layer: Some("meta-demo".into()),
        file: Some(identity.file.clone()),
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert_eq!(
        app.notification.as_deref(),
        Some("Refresh authoritative Devtool status with t before finish.")
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
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert_eq!(
        app.notification.as_deref(),
        Some("Commit all workspace changes before Devtool finish.")
    );
    app.devtool_statuses.get_mut(&identity).unwrap().git = DevtoolGitState::Available {
        repository_root: Some("/build/workspace/sources/busybox".into()),
        branch: Some("devtool".into()),
        upstream: None,
        ahead: 0,
        behind: 0,
        head: Some("abc123".into()),
        modified: 0,
        untracked: 0,
        conflicted: 0,
    };
    let _ = update(&mut app, Action::BeginSelectedRecipeDevtoolFinish);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::DevtoolFinishPicker(picker))
            if picker.identity == identity
                && picker.layers.len() == 2
                && picker.layers[picker.selection] == destination
    ));
    let _ = update(&mut app, Action::PreviewDevtoolFinish);
    let plan = DevtoolFinishPlan {
        identity: identity.clone(),
        layer: destination.clone(),
    };
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::DevtoolFinishConfirmation(plan.clone()))
    );
    assert_eq!(
        update(&mut app, Action::ConfirmDevtoolFinish),
        Some(Effect::DevtoolFinish(plan))
    );

    app.dialogs
        .push_back(Dialog::DevtoolFinishConfirmation(DevtoolFinishPlan {
            identity,
            layer: Layer {
                name: "meta-rogue".into(),
                path: PathBuf::from("/tmp/meta-rogue"),
                priority: None,
            },
        }));
    assert_eq!(update(&mut app, Action::ConfirmDevtoolFinish), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("The selected finish layer is no longer configured.")
    );
}
