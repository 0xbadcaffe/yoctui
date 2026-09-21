use super::*;

#[test]
fn environment_setup_manual_unicode_bounds_and_browser_errors_preserve_draft() {
    let mut app = App::new_unconfigured(20, 2000);
    action(&mut app, EnvironmentSetupAction::Open { browse: false });
    action(&mut app, EnvironmentSetupAction::Edit);
    action(&mut app, EnvironmentSetupAction::Insert("/tmp/café".into()));
    action(&mut app, EnvironmentSetupAction::Backspace);
    assert_eq!(setup(&app).editor.as_ref().unwrap().text, "/tmp/caf");
    action(
        &mut app,
        EnvironmentSetupAction::Insert("\nnot-a-path".into()),
    );
    assert!(setup(&app).error.is_some());
    assert_eq!(setup(&app).editor.as_ref().unwrap().text, "/tmp/caf");
    action(&mut app, EnvironmentSetupAction::Clear);
    action(
        &mut app,
        EnvironmentSetupAction::Insert("/".repeat(ENVIRONMENT_PATH_LIMIT)),
    );
    action(&mut app, EnvironmentSetupAction::Insert("x".into()));
    assert_eq!(
        setup(&app).editor.as_ref().unwrap().text.len(),
        ENVIRONMENT_PATH_LIMIT
    );
    action(&mut app, EnvironmentSetupAction::Cancel);
    let Some(Effect::ReadEnvironmentDirectory { request, .. }) =
        action(&mut app, EnvironmentSetupAction::Browse)
    else {
        panic!("read missing")
    };
    action(
        &mut app,
        EnvironmentSetupAction::DirectoryLoaded {
            request,
            result: Err("Permission denied".into()),
        },
    );
    assert_eq!(setup(&app).error.as_deref(), Some("Permission denied"));
    action(&mut app, EnvironmentSetupAction::ChooseDirectory);
    assert!(setup(&app).values[0].is_empty());
    action(&mut app, EnvironmentSetupAction::Cancel);
    action(&mut app, EnvironmentSetupAction::Cancel);
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Unconfigured
    ));
}
