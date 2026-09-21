use super::*;

#[test]
fn build_environment_form_edits_typed_profile_and_clears_inventory() {
    let mut app = App::new_unconfigured(8, 512);
    app.available_images = vec!["core-image-minimal".into()];
    let _ = update(&mut app, Action::BeginBuildEnvironmentEdit);
    let _ = update(&mut app, Action::AppendBuildEnvironmentField('/'));
    for c in "src".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
    for c in "/build".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::SelectBuildEnvironmentField { delta: 1 });
    for c in "/env".chars() {
        let _ = update(&mut app, Action::AppendBuildEnvironmentField(c));
    }
    let _ = update(&mut app, Action::ApplyBuildEnvironmentProfile);
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Configured(_)
    ));
    assert!(app.available_images.is_empty());
}
