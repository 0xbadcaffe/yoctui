use super::*;

#[test]
fn config_source_rejects_escape_and_explains_unloaded_detail() {
    let mut app = App::new(10, 1_000);
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemuarm".into());
    assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
    assert!(app.notification.as_deref().unwrap().contains("with Enter"));
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
            provenance: None,
            operations: vec![VariableOperation {
                operation: "set".into(),
                file: Some("../outside.conf".into()),
                line: Some(1),
                value: None,
            }],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::OpenSelectedConfigSource);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("escapes the build directory")
    );
}
