use super::*;

#[test]
fn selected_recipe_clean_prefills_the_clean_task() {
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
            tasks: Some(vec!["do_clean".into()]),
            ..RecipeMetadata::default()
        },
    );
    let _ = update(&mut app, Action::BeginSelectedRecipeClean);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest {
            targets,
            task: Some(task),
            force: false,
        })) if targets == &vec!["busybox".to_owned()] && task == "clean"
    ));
}
