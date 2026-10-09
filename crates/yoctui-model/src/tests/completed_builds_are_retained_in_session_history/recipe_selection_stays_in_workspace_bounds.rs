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

#[test]
fn recipe_selection_survives_live_workspace_reordering_by_provider_identity() {
    let mut app = App::new(10, 1_000);
    let chosen = Recipe {
        name: "chosen".into(),
        file: Some("/layer/chosen.bb".into()),
        ..Recipe::default()
    };
    let other = Recipe {
        name: "other".into(),
        file: Some("/layer/other.bb".into()),
        ..Recipe::default()
    };
    app.workspace.recipes = vec![other.clone(), chosen.clone()];
    app.recipe_selection = 1;
    app.metadata_query = "chosen".into();
    let refreshed = Workspace {
        recipes: vec![chosen, other],
        ..Workspace::default()
    };
    let _ = update(&mut app, Action::WorkspaceLoaded(refreshed));
    assert_eq!(app.recipe_selection, 0);
    assert_eq!(app.workspace.recipes[app.recipe_selection].name, "chosen");
    assert_eq!(app.metadata_query, "chosen");
    let _ = update(&mut app, Action::WorkspaceLoaded(Workspace::default()));
    assert_eq!(app.recipe_selection, 0);
}
