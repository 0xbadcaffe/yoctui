use super::*;

#[test]
fn config_scope_falls_back_to_global_when_recipe_disappears() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    let _ = update(&mut app, Action::RecipesLoaded(vec![]));
    assert_eq!(app.config_scope, None);
    let _ = update(&mut app, Action::OpenConfigScopePicker);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigScopePicker(picker)) if picker.scopes == [None]
    ));
}
