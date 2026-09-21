use super::*;

#[test]
fn recipes_workspace_filter_selection_refresh_and_failure_are_identity_stable() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "alpha".into(),
                version: Some("1".into()),
                layer: Some("core".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "busybox".into(),
                version: Some("1.36".into()),
                layer: Some("base".into()),
                file: Some("/layers/base/recipes-core/busybox.bb".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "zlib".into(),
                version: Some("1.3".into()),
                layer: Some("core".into()),
                ..Recipe::default()
            },
        ]),
    );
    let _ = update(&mut app, Action::BeginMetadataSearch);
    for character in "base".chars() {
        let _ = update(&mut app, Action::AppendMetadataQuery(character));
    }
    assert_eq!(app.recipe_selection, 1);
    let _ = update(&mut app, Action::SelectRecipe { delta: isize::MAX });
    assert_eq!(app.recipe_selection, 1);
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeMetadata),
        Some(Effect::GetRecipeMetadata("busybox".into()))
    );
    assert!(app.recipe_metadata_loading.contains("busybox"));
    let _ = update(
        &mut app,
        Action::RecipeMetadataFailed {
            recipe: "busybox".into(),
            message: "server unavailable".into(),
        },
    );
    assert!(!app.recipe_metadata_loading.contains("busybox"));
    assert_eq!(
        app.recipe_metadata_errors
            .get("busybox")
            .map(String::as_str),
        Some("server unavailable")
    );
    app.recipe_metadata_errors
        .insert("alpha".into(), "stale".into());

    let _ = update(
        &mut app,
        Action::RecipesLoaded(vec![
            Recipe {
                name: "busybox".into(),
                version: Some("1.37".into()),
                layer: Some("base".into()),
                ..Recipe::default()
            },
            Recipe {
                name: "new".into(),
                ..Recipe::default()
            },
        ]),
    );
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "busybox");
    assert_eq!(
        app.recipe_metadata_errors
            .get("busybox")
            .map(String::as_str),
        Some("server unavailable")
    );
    assert!(!app.recipe_metadata_errors.contains_key("alpha"));
}
