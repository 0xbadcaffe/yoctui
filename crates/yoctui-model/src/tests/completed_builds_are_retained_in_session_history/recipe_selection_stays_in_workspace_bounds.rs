use super::*;

#[test]
fn recipe_selection_stays_in_workspace_bounds() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "alpha".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        },
        Recipe {
            name: "beta".into(),
            version: None,
            layer: None,
            ..Recipe::default()
        },
    ];
    let _ = update(&mut app, Action::SelectRecipe { delta: 8 });
    assert_eq!(app.recipe_selection, 1);
    let _ = update(&mut app, Action::SelectRecipe { delta: -8 });
    assert_eq!(app.recipe_selection, 0);
}
