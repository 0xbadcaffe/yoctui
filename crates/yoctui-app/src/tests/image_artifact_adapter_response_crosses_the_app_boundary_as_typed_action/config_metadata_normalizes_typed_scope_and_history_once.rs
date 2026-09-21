use super::*;

#[test]
fn config_metadata_normalizes_typed_scope_and_history_once() {
    let action = model_action_from_backend_event(BackendEvent::Variable {
        name: "PACKAGE_ARCH".into(),
        recipe: Some("base-files".into()),
        value: Some("qemux86_64".into()),
        provenance: Some("/layers/meta/conf/machine/qemux86-64.conf:5".into()),
        unexpanded_value: Some("${MACHINE_ARCH}".into()),
        operations: vec![yoctui_model::VariableOperation {
            operation: "set".into(),
            file: Some("/layers/meta/conf/machine/qemux86-64.conf".into()),
            line: Some(5),
            value: Some("${MACHINE_ARCH}".into()),
        }],
        active_overrides: vec!["qemux86-64".into()],
    });
    assert!(matches!(
        action,
        Some(Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity {
                name,
                recipe: Some(recipe),
            },
            unexpanded_value: Some(unexpanded),
            operations,
            ..
        })) if name == "PACKAGE_ARCH"
            && recipe == "base-files"
            && unexpanded == "${MACHINE_ARCH}"
            && operations.len() == 1
    ));
}
