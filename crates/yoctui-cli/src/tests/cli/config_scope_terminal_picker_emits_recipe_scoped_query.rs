use super::*;

#[test]
fn config_scope_terminal_picker_emits_recipe_scoped_query() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "base-files".into(),
        ..yoctui_model::Recipe::default()
    });
    let _ = update(&mut app, Action::OpenConfigScopePicker);
    let _ = config_scope_picker_action(Input::Down).and_then(|action| update(&mut app, action));
    let effect =
        config_scope_picker_action(Input::Enter).and_then(|action| update(&mut app, action));
    assert_eq!(
        effect,
        Some(Effect::GetVariable(VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        }))
    );
    assert_eq!(app.config_scope.as_deref(), Some("base-files"));
}
