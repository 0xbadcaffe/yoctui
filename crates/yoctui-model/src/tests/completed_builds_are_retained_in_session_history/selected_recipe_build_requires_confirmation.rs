use super::*;

#[test]
fn selected_recipe_build_requires_confirmation() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["busybox".into()],
            task: None,
            force: false,
        }))
    );
}
