use super::*;

#[test]
fn config_compare_reports_equal_different_and_unavailable_fields() {
    let mut app = App::new(20, 4_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    app.workspace.recipes.push(Recipe {
        name: "base-files".into(),
        ..Recipe::default()
    });
    app.config_scope = Some("base-files".into());
    for (recipe, effective, unexpanded) in [
        (None, Some("qemux86-64"), Some("${DEFAULT_MACHINE}")),
        (Some("base-files"), Some("qemux86-64"), None),
    ] {
        let identity = VariableIdentity {
            name: "MACHINE".into(),
            recipe: recipe.map(str::to_owned),
        };
        app.variable_details.insert(
            identity.clone(),
            VariableDetail {
                identity,
                effective_value: effective.map(str::to_owned),
                unexpanded_value: unexpanded.map(str::to_owned),
                provenance: None,
                operations: vec![],
                active_overrides: vec![],
            },
        );
    }
    let comparison = config_comparison(&app).unwrap();
    assert_eq!(comparison.effective.outcome, ConfigComparisonOutcome::Equal);
    assert_eq!(
        comparison.unexpanded.outcome,
        ConfigComparisonOutcome::Unavailable
    );
    let _ = update(&mut app, Action::OpenConfigComparison);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::ConfigComparison(value)) if value == &comparison
    ));
    let _ = update(&mut app, Action::CloseConfigComparison);
    assert!(app.active_dialog().is_none());

    app.variable_details
        .get_mut(&VariableIdentity {
            name: "MACHINE".into(),
            recipe: Some("base-files".into()),
        })
        .unwrap()
        .effective_value = Some("qemuarm".into());
    assert_eq!(
        config_comparison(&app).unwrap().effective.outcome,
        ConfigComparisonOutcome::Different
    );
}
