use super::*;

#[test]
fn raw_selector_app_projection_replaces_workspace_and_target_inventories() {
    let mut app = yoctui_model::App::new(10, 1_000);
    let absent = raw_selector_authority(&app, None);
    assert!(matches!(
        absent.recipes,
        yoctui_model::RawSelectorInventory::Unavailable { .. }
    ));

    app.workspace.build_dir = Some("/build".into());
    app.workspace.recipes = vec![yoctui_model::Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox/busybox.bb".into()),
        ..yoctui_model::Recipe::default()
    }];
    app.available_images = vec!["core-image-minimal".into()];
    app.workspace
        .variables
        .insert("BBMULTICONFIG".into(), "lib32 board1".into());
    app.build.target = Some("core-image-minimal".into());
    app.build_history.push_back(yoctui_model::BuildRecord {
        target: Some("busybox".into()),
        success: true,
        exit_code: Some(0),
        elapsed: Some(std::time::Duration::from_secs(1)),
        completed_tasks: 1,
        warnings: 0,
        errors: 0,
    });

    let initial = raw_selector_authority(&app, None);
    assert_eq!(initial.recipes.choices().unwrap().len(), 1);
    assert_eq!(initial.images.choices().unwrap().len(), 1);
    assert_eq!(initial.targets.choices().unwrap().len(), 2);
    assert_eq!(initial.multiconfigs.choices().unwrap().len(), 2);

    app.workspace.recipes = vec![yoctui_model::Recipe {
        name: "bash".into(),
        file: Some("/layers/meta/recipes-extended/bash/bash.bb".into()),
        ..yoctui_model::Recipe::default()
    }];
    app.available_images.clear();
    app.build.target = Some("bash".into());
    app.build_history.clear();
    let replaced = raw_selector_authority(&app, None);
    assert_eq!(
        replaced.recipes.choices().unwrap()[0].value,
        yoctui_model::RawParameterValue::Recipe("bash".into())
    );
    assert_eq!(replaced.images.choices().unwrap().len(), 0);
    assert_eq!(replaced.targets.choices().unwrap().len(), 1);
}
