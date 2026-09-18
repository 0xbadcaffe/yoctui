use super::*;

#[test]
fn project_profile_summary_keeps_resolution_explicit() {
    let mut app = App::new(16, 4096);
    app.project_profile = yoctui_model::ProjectProfileState::Loaded(project_profile_fixture());
    app.workspace.recipes = vec![yoctui_model::Recipe {
        name: "core-image-minimal".into(),
        ..yoctui_model::Recipe::default()
    }];
    app.available_images = vec!["core-image-minimal".into()];
    assert_eq!(
        project_profile_summary(&app),
        vec![
            "project profile: loaded",
            "profile item: resolved FavoriteImage(0)",
        ]
    );

    app.available_images.clear();
    assert!(project_profile_summary(&app)[1].contains("stale"));
}
