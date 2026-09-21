use super::*;

#[test]
fn config_workspace_lazy_detail_is_identity_correlated_and_search_bounded() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "poky".into());
    app.metadata_query = "machine".into();
    let identity = match update(&mut app, Action::BeginSelectedConfigDetail) {
        Some(Effect::GetVariable(identity)) => identity,
        effect => panic!("unexpected effect: {effect:?}"),
    };
    assert_eq!(identity.name, "MACHINE");
    assert_eq!(identity.recipe, None);
    assert!(app.variable_detail_loading.contains(&identity));

    let scoped = VariableDetail {
        identity: VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        },
        effective_value: Some("qemux86-64".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    let _ = update(&mut app, Action::VariableLoaded(scoped.clone()));
    assert!(
        app.variable_detail_loading.contains(&identity),
        "a scoped response must not complete the selected global request"
    );
    assert_eq!(app.variable_details.get(&scoped.identity), Some(&scoped));

    let _ = update(&mut app, Action::SelectConfigVariable { delta: 99 });
    assert_eq!(app.config_selection, 0);
    let _ = update(
        &mut app,
        Action::VariableDetailFailed {
            identity: identity.clone(),
            message: "server unavailable".into(),
        },
    );
    assert!(!app.variable_detail_loading.contains(&identity));
    assert_eq!(
        app.variable_detail_errors
            .get(&identity)
            .map(String::as_str),
        Some("server unavailable")
    );
}
