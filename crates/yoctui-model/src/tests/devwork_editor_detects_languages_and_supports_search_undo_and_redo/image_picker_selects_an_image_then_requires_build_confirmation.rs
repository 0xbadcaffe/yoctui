use super::*;

#[test]
fn image_picker_selects_an_image_then_requires_build_confirmation() {
    let mut app = App::new(10, 1_000);
    let _ = update(
        &mut app,
        Action::OpenImagePicker(vec!["core-image-base".into(), "core-image-minimal".into()]),
    );
    let _ = update(&mut app, Action::SelectImage { delta: 1 });
    let _ = update(&mut app, Action::ConfirmImagePicker);
    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    let _ = update(&mut app, Action::BeginCurrentImageBuild);
    assert_eq!(
        app.active_dialog(),
        Some(&Dialog::RecipeTaskConfirmation(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: None,
            force: false,
        }))
    );
}

#[test]
fn build_image_chooses_a_recipe_in_place_before_showing_options() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "core-image-base".into(),
            file: Some("/layers/core-image-base.bb".into()),
            ..Recipe::default()
        },
        Recipe {
            name: "core-image-minimal".into(),
            file: Some("/layers/core-image-minimal.bb".into()),
            ..Recipe::default()
        },
    ];

    let _ = update(&mut app, Action::OpenBuildOptions);
    assert!(matches!(app.active_dialog(), Some(Dialog::ImagePicker(_))));
    let _ = update(&mut app, Action::SelectImage { delta: 1 });
    let _ = update(&mut app, Action::ConfirmImagePicker);

    assert_eq!(app.build.target.as_deref(), Some("core-image-minimal"));
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
}

#[test]
fn recipe_operations_choose_their_recipe_without_leaving_the_current_flow() {
    let mut app = App::new(10, 1_000);
    app.workspace.recipes = vec![
        Recipe {
            name: "busybox".into(),
            file: Some("/layers/busybox.bb".into()),
            ..Recipe::default()
        },
        Recipe {
            name: "systemd".into(),
            file: Some("/layers/systemd.bb".into()),
            ..Recipe::default()
        },
    ];
    app.screen = Screen::Dependencies;

    let _ = update(&mut app, Action::RefreshDependencyGraph);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipePicker(RecipePicker {
            purpose: RecipePickerPurpose::Dependencies,
            ..
        }))
    ));
    let _ = update(&mut app, Action::SelectRecipePicker { delta: 1 });
    assert_eq!(
        update(&mut app, Action::ConfirmRecipePicker),
        Some(Effect::GetDependencies("systemd".into()))
    );
    assert!(app.active_dialog().is_none());
    assert_eq!(app.screen, Screen::Dependencies);

    app.screen = Screen::Dashboard;
    let _ = update(&mut app, Action::BeginSelectedRecipeBuild);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipePicker(RecipePicker {
            purpose: RecipePickerPurpose::Build,
            ..
        }))
    ));
    let _ = update(&mut app, Action::ConfirmRecipePicker);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeTaskConfirmation(BuildRequest { targets, .. }))
            if targets == &vec!["systemd".to_owned()]
    ));
}
