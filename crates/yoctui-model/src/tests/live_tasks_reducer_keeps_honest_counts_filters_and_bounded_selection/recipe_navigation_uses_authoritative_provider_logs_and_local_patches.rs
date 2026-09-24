use super::*;

#[test]
fn recipe_navigation_uses_authoritative_provider_logs_and_local_patches() {
    let mut app = App::new(20, 4_000);
    app.workspace.recipes.push(Recipe {
        name: "busybox".into(),
        file: Some("/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()),
        ..Recipe::default()
    });
    app.recipe_metadata.insert(
        "busybox".into(),
        RecipeMetadata {
            recipe: "busybox".into(),
            patches: Some(vec![
                "/layers/meta/recipes-core/busybox/files/a.patch".into(),
                "https://example.invalid/remote.diff".into(),
                "/layers/meta/recipes-core/busybox/files/b.patch".into(),
            ]),
            ..RecipeMetadata::default()
        },
    );
    let mut active = TaskInfo::active(
        TaskId("busybox:do_compile".into()),
        "busybox".into(),
        "do_compile".into(),
    );
    active.log_path = Some("/tmp/log.do_compile".into());
    app.tasks.insert(active.id.clone(), active);
    let mut completed = TaskInfo::active(
        TaskId("busybox:do_install".into()),
        "busybox".into(),
        "do_install".into(),
    );
    completed.state = TaskState::Completed;
    completed.log_path = Some("/tmp/log.do_install".into());
    app.completed_tasks.push_back(CompletedTask {
        task: completed,
        success: true,
    });

    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipeProvider),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes-core/busybox/busybox_1.0.bb".into()
        ))
    );
    assert_eq!(update(&mut app, Action::BeginSelectedRecipeTaskLog), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskLogPicker(picker)) if picker.logs.len() == 2
    ));
    let _ = update(&mut app, Action::SelectRecipeTaskLog { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipeTaskLog),
        Some(Effect::OpenInEditor("/tmp/log.do_install".into()))
    );

    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipePatchReview),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipePatchPicker(picker)) if picker.patches.len() == 2
    ));
    let _ = update(&mut app, Action::SelectRecipePatch { delta: 1 });
    assert_eq!(
        update(&mut app, Action::OpenSelectedRecipePatch),
        Some(Effect::OpenInEditor(
            "/layers/meta/recipes-core/busybox/files/b.patch".into()
        ))
    );

    app.recipe_metadata.get_mut("busybox").unwrap().patches =
        Some(vec!["/layers/meta/files/only.patch".into()]);
    assert_eq!(
        update(&mut app, Action::BeginSelectedRecipePatchReview),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipePatchPicker(picker))
            if picker.patches == vec![PathBuf::from("/layers/meta/files/only.patch")]
    ));
}
