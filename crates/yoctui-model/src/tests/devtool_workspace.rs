use super::*;

#[test]
fn devtool_workspace_is_a_distinct_navigator_screen_with_devtool_authority() {
    for (destination, expected) in [
        (WorkspaceDestination::Recipes, Screen::Recipes),
        (WorkspaceDestination::Devtool, Screen::Devtool),
    ] {
        let index = App::navigator_selection_for_destination(destination).unwrap();
        assert_eq!(NAVIGATOR_SCREENS[index], expected);
    }
    assert_eq!(
        workspace_screen_destination(Screen::Devtool),
        WorkspaceDestination::Devtool
    );

    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Open(Screen::Devtool));
    assert_eq!(app.screen, Screen::Devtool);
    app.focus = FocusTarget::Workspace;
    assert_eq!(app.inspector_mode(), InspectorMode::Recipe);
}

#[test]
fn devtool_workspace_builds_the_exact_selected_recipe_without_leaving_the_screen() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Devtool;
    app.focus = FocusTarget::Workspace;
    app.workspace.recipes.push(Recipe {
        name: "phosphor-state-manager".into(),
        file: Some("/work/meta/recipes/phosphor-state-manager.bb".into()),
        ..Recipe::default()
    });

    let effect = update(&mut app, Action::BeginSelectedRecipeBuild);
    assert_eq!(effect, None);
    assert_eq!(app.screen, Screen::Devtool);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest { targets, task: None, force: false }))
            if targets == &["phosphor-state-manager".to_owned()]
    ));
}
