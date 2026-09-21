use super::*;

#[test]
fn recipe_metadata_refresh_is_typed_and_replaces_stale_detail() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        version: Some("1.36".into()),
        layer: Some("core".into()),
        preferred_version: None,
        file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
        append_count: Some(2),
    });
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeMetadata),
        Some(Effect::GetRecipeMetadata("busybox".into()))
    );
    let _ = update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "busybox".into(),
            workspace_status: None,
            build_status: None,
            tasks: Some(vec!["do_build".into()]),
            sources: Some(vec!["/layers/meta/busybox.bb".into()]),
            patches: Some(vec![]),
            packages: Some(vec!["busybox".into()]),
            history: None,
        }),
    );
    let _ = update(
        &mut app,
        Action::RecipeMetadataLoaded(RecipeMetadata {
            recipe: "busybox".into(),
            tasks: Some(vec!["do_compile".into()]),
            sources: None,
            ..RecipeMetadata::default()
        }),
    );
    let metadata = &app.recipe_metadata["busybox"];
    assert_eq!(metadata.tasks, Some(vec!["do_compile".into()]));
    assert_eq!(metadata.sources, None);
    assert!(!app.recipe_sources.contains_key("busybox"));
    assert_eq!(metadata.workspace_status, None);
    assert_eq!(metadata.history, None);
}
