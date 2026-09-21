use super::*;

#[test]
fn config_metadata_converts_scope_unexpanded_value_and_operations() {
    let event = Event::Variable {
        name: "MACHINE".into(),
        recipe: Some("base-files".into()),
        value: Some("qemux86-64".into()),
        provenance: Some("/build/conf/local.conf:12".into()),
        unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
        operations: vec![yoctui_protocol::VariableOperationData {
            operation: "set".into(),
            file: Some("/build/conf/local.conf".into()),
            line: Some(12),
            value: Some("${DEFAULT_MACHINE}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    };
    let BackendEvent::Variable {
        name,
        recipe,
        value,
        unexpanded_value,
        operations,
        active_overrides,
        ..
    } = BridgeBackend::event(event).unwrap()
    else {
        panic!("variable detail event was not preserved");
    };
    assert_eq!(name, "MACHINE");
    assert_eq!(recipe.as_deref(), Some("base-files"));
    assert_eq!(value.as_deref(), Some("qemux86-64"));
    assert_eq!(unexpanded_value.as_deref(), Some("${DEFAULT_MACHINE}"));
    assert_eq!(
        operations[0].file,
        Some(PathBuf::from("/build/conf/local.conf"))
    );
    assert_eq!(operations[0].line, Some(12));
    assert_eq!(active_overrides, ["qemux86-64"]);
}
