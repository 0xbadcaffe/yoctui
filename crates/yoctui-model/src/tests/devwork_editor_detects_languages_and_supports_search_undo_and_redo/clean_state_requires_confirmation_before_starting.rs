use super::*;

#[test]
fn clean_state_requires_confirmation_before_starting() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_cleansstate".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeCleanState);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(_))
    ));
    assert_eq!(app.build.status, BuildStatus::Idle);

    assert_eq!(
        update(&mut app, Action::ConfirmRecipeTask),
        Some(Effect::Start(BuildRequest {
            targets: vec!["busybox".into()],
            task: Some("cleansstate".into()),
            force: false,
        }))
    );
    assert_eq!(app.build.status, BuildStatus::LoadingWorkspace);
}
