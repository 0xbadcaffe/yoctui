use super::*;

#[test]
fn recipe_bitbake_action_rejects_unavailable_and_malformed_tasks() {
    let mut app = App::new(10, 1_000);
    app.daemon.status = ClientReplicaStatus::Current;
    app.workspace.build_dir = Some("/work/build".into());
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::BeginSelectedRecipeDevshell);
    assert_eq!(
        app.notification.as_deref(),
        Some("Load authoritative recipe tasks with Enter before opening an interactive task.")
    );
    app.recipe_metadata.insert(
        "demo".into(),
        RecipeMetadata {
            recipe: "demo".into(),
            tasks: Some(vec!["do_build".into(), "bad task".into()]),
            ..RecipeMetadata::default()
        },
    );
    app.notification = None;
    let _ = update(&mut app, Action::BeginSelectedRecipeMenuConfig);
    assert_eq!(
        app.notification.as_deref(),
        Some("Task menuconfig is not reported for recipe demo.")
    );
    app.notification = None;
    let _ = update(
        &mut app,
        Action::BeginSelectedRecipeTask {
            task: Some("bad task".into()),
            force: true,
        },
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("invalid build target"))
    );
    assert!(app.active_dialog().is_none());
}
