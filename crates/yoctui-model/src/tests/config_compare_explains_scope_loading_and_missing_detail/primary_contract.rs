use super::*;

#[test]
fn config_compare_explains_scope_loading_and_missing_detail() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    assert_eq!(
        config_comparison(&app),
        Err("Select a recipe scope with s before comparing.".into())
    );
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    let scoped = VariableIdentity {
        name: "MACHINE".into(),
        recipe: Some("base-files".into()),
    };
    app.variable_detail_loading.insert(scoped);
    assert!(
        config_comparison(&app)
            .unwrap_err()
            .contains("still loading")
    );
}
