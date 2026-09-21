use super::*;

#[test]
fn config_edit_write_revalidates_request_and_preserves_detail_on_failures() {
    let identity = VariableIdentity {
        name: "MACHINE".into(),
        recipe: None,
    };
    let request = ConfigEditRequest {
        identity: identity.clone(),
        value: "qemux86-64".into(),
        destination: "/build/conf/local.conf".into(),
        assignment: "MACHINE = \"qemux86-64\"".into(),
    };
    assert_eq!(
        validate_config_edit_request(&request, Path::new("/build")),
        Ok(())
    );

    let mut tampered = request.clone();
    tampered.assignment = "MACHINE = \"injected\"".into();
    assert!(
        validate_config_edit_request(&tampered, Path::new("/build"))
            .unwrap_err()
            .contains("does not match")
    );
    let mut scoped = request.clone();
    scoped.identity.recipe = Some("base-files".into());
    assert!(
        validate_config_edit_request(&scoped, Path::new("/build"))
            .unwrap_err()
            .contains("Recipe-scoped")
    );

    let mut app = App::new(10, 1_000);
    let detail = VariableDetail {
        identity: identity.clone(),
        effective_value: Some("old".into()),
        unexpanded_value: None,
        provenance: None,
        operations: vec![],
        active_overrides: vec![],
    };
    app.variable_details
        .insert(identity.clone(), detail.clone());
    assert_eq!(
        update(
            &mut app,
            Action::ConfigEditWriteSucceeded {
                identity: identity.clone(),
            },
        ),
        Some(Effect::GetVariable(identity.clone()))
    );
    assert!(app.variable_detail_loading.contains(&identity));
    let _ = update(
        &mut app,
        Action::ConfigEditRefreshFailed {
            identity: identity.clone(),
            message: "bridge unavailable".into(),
        },
    );
    assert_eq!(app.variable_details.get(&identity), Some(&detail));
    assert!(!app.variable_detail_loading.contains(&identity));

    let _ = update(
        &mut app,
        Action::ConfigEditWriteFailed {
            identity: identity.clone(),
            message: "permission denied".into(),
        },
    );
    assert_eq!(app.variable_details.get(&identity), Some(&detail));
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("permission denied")
    );
}
