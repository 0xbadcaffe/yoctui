use super::*;

#[test]
fn config_scope_keeps_global_and_recipe_detail_independent() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    let global = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        global.clone(),
        VariableDetail {
            identity: global.clone(),
            effective_value: Some("global-machine".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::OpenConfigScopePicker);
    let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() else {
        panic!("scope picker was not opened");
    };
    assert_eq!(picker.scopes, [None, Some("base-files".into())]);
    let _ = update(&mut app, Action::SelectConfigScope { delta: 1 });
    let scoped = match update(&mut app, Action::ConfirmConfigScope) {
        Some(Effect::GetVariable(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert_eq!(scoped.recipe.as_deref(), Some("base-files"));
    assert!(app.variable_detail_loading.contains(&scoped));
    assert_eq!(
        app.variable_details[&global].effective_value.as_deref(),
        Some("global-machine")
    );
    let _ = update(
        &mut app,
        Action::VariableLoaded(VariableDetail {
            identity: scoped.clone(),
            effective_value: Some("recipe-machine".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        }),
    );
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigEffective),
        Some(Effect::CopyToClipboard("recipe-machine".into()))
    );
}
