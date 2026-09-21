use super::*;

#[test]
fn selected_recipe_requests_authoritative_dependencies() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![Recipe {
        name: "busybox".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    }];
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipeDependencies),
        Some(Effect::GetDependencies("busybox".into()))
    );
    let _ = update(
        &mut app,
        Action::DependenciesLoaded(RecipeDependencies {
            recipe: "busybox".into(),
            build: vec!["virtual/libc".into()],
            runtime: vec!["base-files".into()],
        }),
    );
    assert_eq!(app.screen, Screen::Dependencies);
    assert_eq!(app.dependencies.as_ref().unwrap().build, ["virtual/libc"]);
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        version: None,
        layer: None,
        ..Recipe::default()
    });
    let _ = update(&mut app, Action::SelectDependency { delta: 1 });
    let _ = update(&mut app, Action::OpenSelectedDependency);
    assert_eq!(app.screen, Screen::Recipes);
    assert_eq!(app.recipe_selection, 1);
}
