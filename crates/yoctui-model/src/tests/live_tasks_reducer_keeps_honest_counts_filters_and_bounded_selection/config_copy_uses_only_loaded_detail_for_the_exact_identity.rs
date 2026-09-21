use super::*;

#[test]
fn config_copy_uses_only_loaded_detail_for_the_exact_identity() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "summary-value".into());
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigEffective),
        None,
        "the summary value must not be copied as authoritative detail"
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("with Enter"))
    );
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
        update(&mut app, Action::CopySelectedConfigEffective),
        Some(Effect::CopyToClipboard("qemux86-64".into()))
    );
    assert_eq!(
        update(&mut app, Action::CopySelectedConfigUnexpanded),
        Some(Effect::CopyToClipboard("${DEFAULT_MACHINE}".into()))
    );
}
