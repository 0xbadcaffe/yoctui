use super::*;

#[test]
fn config_metadata_keeps_global_and_recipe_scopes_typed_and_independent() {
    let mut app = App::new(20, 4_000);
    let global = VariableDetail {
        identity: VariableIdentity {
            name: "PACKAGE_ARCH".into(),
            recipe: None,
        },
        effective_value: Some("qemux86_64".into()),
        unexpanded_value: Some("${MACHINE_ARCH}".into()),
        provenance: Some("/build/conf/local.conf:8".into()),
        operations: vec![VariableOperation {
            operation: "set".into(),
            file: Some("/build/conf/local.conf".into()),
            line: Some(8),
            value: Some("${MACHINE_ARCH}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    };
    let _ = update(&mut app, Action::VariableLoaded(global.clone()));
    assert_eq!(app.workspace.variables["PACKAGE_ARCH"], "qemux86_64");
    assert_eq!(
        app.workspace.variable_provenance_chain["PACKAGE_ARCH"],
        ["/build/conf/local.conf:8"]
    );

    let recipe = VariableDetail {
        identity: VariableIdentity {
            name: "PACKAGE_ARCH".into(),
            recipe: Some("base-files".into()),
        },
        effective_value: Some("all".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    let _ = update(&mut app, Action::VariableLoaded(recipe.clone()));
    assert_eq!(
        app.workspace.variables["PACKAGE_ARCH"], "qemux86_64",
        "a scoped response must not overwrite global summary state"
    );
    assert_eq!(
        app.variable_details
            .get(&recipe.identity)
            .unwrap()
            .effective_value
            .as_deref(),
        Some("all")
    );
    assert_eq!(app.variable_details.get(&global.identity), Some(&global));
}
