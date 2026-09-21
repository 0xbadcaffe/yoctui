use super::*;

#[test]
fn recipe_navigation_explains_missing_and_remote_only_paths() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "demo".into(),
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::OpenSelectedRecipeProvider);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("provider path")
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeTaskLog);
    assert!(app.notification.as_deref().unwrap().contains("evicted"));
    app.recipe_metadata.insert(
        "demo".into(),
        RecipeMetadata {
            recipe: "demo".into(),
            patches: Some(vec!["file://unresolved.patch".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipePatchReview);
    assert!(app.notification.as_deref().unwrap().contains("unresolved"));
}
