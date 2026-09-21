use super::*;

#[test]
fn raw_selector_app_projection_rejects_stale_tasks_but_keeps_valid_manual_entry() {
    let mut app = yoctui_model::App::new(10, 1_000);
    app.recipe_metadata.insert(
        "alpha".into(),
        yoctui_model::RecipeMetadata {
            recipe: "alpha".into(),
            tasks: Some(vec!["do_compile".into()]),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let (command, parameter) = raw_selector_command(yoctui_model::RawParameterKind::Task);

    let alpha = raw_selector_for_command(&app, &command, &parameter, Some("alpha")).unwrap();
    assert_eq!(alpha.inventory.choices().unwrap().len(), 1);

    let beta = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert!(matches!(
        beta.inventory,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));
    assert!(beta.manual_entry);
    assert_eq!(
        beta.parse_manual("do_install").unwrap(),
        Some(yoctui_model::RawParameterValue::Task("do_install".into()))
    );
    assert!(beta.parse_manual("do_install;touch").is_err());

    app.recipe_metadata.insert(
        "beta".into(),
        yoctui_model::RecipeMetadata {
            recipe: "beta".into(),
            tasks: Some(Vec::new()),
            ..yoctui_model::RecipeMetadata::default()
        },
    );
    let empty = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert_eq!(empty.inventory.choices().unwrap().len(), 0);

    app.recipe_metadata_loading.insert("beta".into());
    let loading = raw_selector_for_command(&app, &command, &parameter, Some("beta")).unwrap();
    assert!(matches!(
        loading.inventory,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));
}
