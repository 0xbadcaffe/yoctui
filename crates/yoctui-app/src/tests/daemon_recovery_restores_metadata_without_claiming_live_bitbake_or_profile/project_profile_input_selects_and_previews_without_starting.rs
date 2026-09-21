use super::*;

#[test]
fn project_profile_input_selects_and_previews_without_starting() {
    assert_eq!(
        build_environment_action(Input::Char('n')),
        Some(Action::SelectProjectProfileItem { delta: 1 })
    );
    assert_eq!(
        build_environment_action(Input::Char('p')),
        Some(Action::ActivateProjectProfileItem)
    );
    let profile = yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites::default(),
        build_presets: vec![yoctui_model::ProjectBuildPreset {
            name: "minimal".into(),
            targets: vec!["core-image-minimal".into()],
            machine: None,
            distro: None,
            options: yoctui_model::ProjectBuildOptions::default(),
        }],
        workflows: Vec::new(),
    };
    let mut app = App::new(8, 512);
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: "core-image-minimal".into(),
        ..yoctui_model::Recipe::default()
    });
    let _ = update(&mut app, Action::ProjectProfileLoaded(profile));
    assert_eq!(update(&mut app, Action::ActivateProjectProfileItem), None);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeTaskConfirmation(request))
            if request.targets == ["core-image-minimal"]
    ));
    assert!(matches!(app.build.status, yoctui_model::BuildStatus::Idle));
}
