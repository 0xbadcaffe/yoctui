use super::*;

#[test]
fn config_source_opens_single_typed_relative_operation() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some(PathBuf::from("/build"));
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemuarm".into()),
            unexpanded_value: None,
            provenance: Some("conf/local.conf:12".into()),
            operations: vec![VariableOperation {
                operation: "set".into(),
                file: Some("conf/local.conf".into()),
                line: Some(12),
                value: Some("qemuarm".into()),
            }],
            active_overrides: vec![],
        },
    );
    assert_eq!(
        update(&mut app, Action::OpenSelectedConfigSource),
        Some(Effect::OpenInEditor(PathBuf::from(
            "/build/conf/local.conf"
        )))
    );
}
