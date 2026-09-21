use super::*;

#[test]
fn config_edit_preview_rejects_read_only_scope_and_control_injection() {
    let mut app = App::new(20, 4_000);
    app.workspace.build_dir = Some("/build".into());
    app.workspace
        .variables
        .insert("BB_NUMBER_THREADS".into(), "8".into());
    let identity = VariableIdentity {
        name: "BB_NUMBER_THREADS".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("8".into()),
            unexpanded_value: None,
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    let _ = update(&mut app, Action::BeginConfigEdit);
    assert!(app.notification.as_deref().unwrap().contains("read-only"));
    assert!(app.active_dialog().is_none());

    assert_eq!(
        config_edit_assignment("MACHINE", "qemu\nMALICIOUS = \"1\""),
        Err("Configuration values cannot contain newlines or control characters.".into())
    );
    app.config_scope = Some("base-files".into());
    assert!(
        config_edit_disabled_reason(&app)
            .unwrap()
            .contains("Recipe-scoped")
    );
}
