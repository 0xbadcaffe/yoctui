use super::*;

#[test]
fn config_copy_explains_loading_failure_and_absent_unexpanded_value() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_detail_loading.insert(identity.clone());
    assert_eq!(update(&mut app, Action::CopySelectedConfigEffective), None);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("still loading")
    );
    app.variable_detail_loading.clear();
    app.variable_detail_errors
        .insert(identity.clone(), "Tinfoil unavailable".into());
    let _ = update(&mut app, Action::CopySelectedConfigEffective);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("Tinfoil unavailable")
    );
    app.variable_detail_errors.clear();
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::CopySelectedConfigUnexpanded);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("unexpanded value")
    );
}
