use super::*;

#[test]
fn config_workspace_terminal_result_preserves_typed_scope_and_history() {
    let action = config_variable_loaded_action(
        VariableIdentity {
            name: "MACHINE".into(),
            recipe: None,
        },
        VariableValue {
            recipe: Some("base-files".into()),
            value: Some("qemux86-64".into()),
            provenance: Some("/build/conf/local.conf:3".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            operations: vec![yoctui_model::VariableOperation {
                operation: "set".into(),
                file: Some("/build/conf/local.conf".into()),
                line: Some(3),
                value: Some("${DEFAULT_MACHINE}".into()),
            }],
            active_overrides: vec!["qemux86-64".into()],
        },
    );
    assert!(matches!(
        action,
        Action::VariableLoaded(VariableDetail {
            identity: VariableIdentity {
                name,
                recipe: Some(recipe),
            },
            unexpanded_value: Some(unexpanded),
            operations,
            active_overrides,
            ..
        }) if name == "MACHINE"
            && recipe == "base-files"
            && unexpanded == "${DEFAULT_MACHINE}"
            && operations.len() == 1
            && active_overrides == ["qemux86-64"]
    ));
}
