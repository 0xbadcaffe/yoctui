use super::*;

#[test]
fn config_copy_terminal_route_emits_existing_clipboard_effect() {
    let mut app = App::new(10, 1_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    app.variable_details.insert(
        identity.clone(),
        VariableDetail {
            identity,
            effective_value: Some("qemux86-64".into()),
            unexpanded_value: Some("${DEFAULT_MACHINE}".into()),
            provenance: None,
            operations: vec![],
            active_overrides: vec![],
        },
    );
    assert_eq!(
        config_copy_effect(&mut app, Input::Char('C')),
        Some(Effect::CopyToClipboard("qemux86-64".into()))
    );
    assert_eq!(
        config_copy_effect(&mut app, Input::Char('U')),
        Some(Effect::CopyToClipboard("${DEFAULT_MACHINE}".into()))
    );
}
