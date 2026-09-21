use super::*;

#[test]
fn environment_setup_manual_save_cancel_and_validation() {
    let mut app = App::new(20, 2000);
    app.build_environment = BuildEnvironmentState::Unconfigured;
    action(&mut app, EnvironmentSetupAction::Open { browse: false });
    action(&mut app, EnvironmentSetupAction::Save);
    assert!(setup(&app).error.is_some());
    for value in [
        "/tmp/source with spaces",
        "/tmp/new-build",
        "/tmp/source with spaces/oe-init-build-env",
    ] {
        action(&mut app, EnvironmentSetupAction::Edit);
        action(&mut app, EnvironmentSetupAction::Insert(value.into()));
        action(&mut app, EnvironmentSetupAction::AcceptEdit);
        action(&mut app, EnvironmentSetupAction::Field(1));
    }
    action(&mut app, EnvironmentSetupAction::Save);
    assert!(
        matches!(&app.build_environment, BuildEnvironmentState::Configured(p) if p.build_dir == std::path::Path::new("/tmp/new-build"))
    );
    let before = app.build_environment.clone();
    action(&mut app, EnvironmentSetupAction::Open { browse: false });
    action(&mut app, EnvironmentSetupAction::Edit);
    action(
        &mut app,
        EnvironmentSetupAction::Insert("/elsewhere".into()),
    );
    action(&mut app, EnvironmentSetupAction::Cancel);
    action(&mut app, EnvironmentSetupAction::Cancel);
    assert_eq!(app.build_environment, before);
}
