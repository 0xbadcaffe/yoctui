use super::*;

#[test]
fn config_edit_write_refresh_success_replaces_detail_and_failure_preserves_it() {
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    let mut app = App::new(10, 1_000);
    let old = VariableDetail {
        identity: identity.clone(),
        effective_value: Some("old".into()),
        unexpanded_value: None,
        provenance: Some("conf/local.conf:1".into()),
        operations: vec![],
        active_overrides: vec![],
    };
    app.variable_details.insert(identity.clone(), old.clone());
    let _ = update(
        &mut app,
        Action::ConfigEditWriteSucceeded {
            identity: identity.clone(),
        },
    );
    finish_config_edit_refresh(
        &mut app,
        identity.clone(),
        Ok(VariableValue {
            value: Some("qemux86-64".into()),
            provenance: Some("conf/local.conf:1".into()),
            ..VariableValue::default()
        }),
    );
    assert_eq!(
        app.variable_details[&identity].effective_value.as_deref(),
        Some("qemux86-64")
    );
    assert_eq!(
        app.notification.as_deref(),
        Some("MACHINE saved and refreshed.")
    );

    app.variable_details.insert(identity.clone(), old.clone());
    let _ = update(
        &mut app,
        Action::ConfigEditWriteSucceeded {
            identity: identity.clone(),
        },
    );
    finish_config_edit_refresh(
        &mut app,
        identity.clone(),
        Err(yoctui_bitbake::BackendError::Bridge("offline".into())),
    );
    assert_eq!(app.variable_details.get(&identity), Some(&old));
    assert!(!app.variable_detail_loading.contains(&identity));
    assert!(app.notification.as_deref().unwrap().contains("offline"));
}
