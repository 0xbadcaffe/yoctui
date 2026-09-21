use super::*;

#[test]
fn project_profile_generation_crosses_app_boundary_as_typed_effect() {
    let profile = yoctui_model::ProjectProfile {
        schema_version: yoctui_model::PROJECT_PROFILE_SCHEMA_VERSION,
        favorites: yoctui_model::ProjectFavorites::default(),
        build_presets: Vec::new(),
        workflows: Vec::new(),
    };
    let mut app = App::new(8, 512);
    assert_eq!(
        update(
            &mut app,
            Action::PreviewProjectProfileGeneration(profile.clone())
        ),
        None
    );
    assert_eq!(
        update(
            &mut app,
            Action::ConfirmProjectProfileGeneration { replace: false }
        ),
        Some(yoctui_model::Effect::GenerateProjectProfile {
            profile: profile.clone(),
            replace: false,
        })
    );
    let _ = update(&mut app, Action::ProjectProfileGenerated(profile.clone()));
    assert_eq!(
        app.project_profile,
        yoctui_model::ProjectProfileState::Loaded(profile)
    );
}
