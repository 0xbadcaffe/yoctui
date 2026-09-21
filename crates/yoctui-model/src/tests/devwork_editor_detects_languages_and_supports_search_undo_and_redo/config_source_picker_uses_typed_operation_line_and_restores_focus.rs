use super::*;

#[test]
fn config_source_picker_uses_typed_operation_line_and_restores_focus() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Configuration;
    app.focus = FocusTarget::Inspector;
    app.workspace.build_dir = Some("/build".into());
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
            provenance: None,
            operations: vec![
                VariableOperation {
                    operation: "set".into(),
                    file: Some("meta/conf/bitbake.conf".into()),
                    line: Some(10),
                    value: None,
                },
                VariableOperation {
                    operation: "override".into(),
                    file: Some("conf/local.conf".into()),
                    line: Some(12),
                    value: None,
                },
            ],
            active_overrides: vec![],
        },
    );
    assert_eq!(update(&mut app, Action::OpenSelectedConfigSource), None);
    assert_eq!(app.focus, FocusTarget::Dialog);
    let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() else {
        panic!("source picker was not opened");
    };
    assert_eq!(picker.sources[1].operation, "override");
    assert_eq!(picker.sources[1].line, Some(12));
    let _ = update(&mut app, Action::SelectConfigSource { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedConfigSourceChoice),
        Some(Effect::OpenInEditor("/build/conf/local.conf".into()))
    );
    assert_eq!(app.focus, FocusTarget::Navigator);
}
