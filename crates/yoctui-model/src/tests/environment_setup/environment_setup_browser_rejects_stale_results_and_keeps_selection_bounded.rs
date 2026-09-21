use super::*;

#[test]
fn environment_setup_browser_rejects_stale_results_and_keeps_selection_bounded() {
    let mut app = App::new(20, 2000);
    let Some(Effect::ReadEnvironmentDirectory { request, .. }) =
        action(&mut app, EnvironmentSetupAction::Open { browse: true })
    else {
        panic!("read missing")
    };
    action(&mut app, EnvironmentSetupAction::Cancel);
    action(&mut app, EnvironmentSetupAction::Browse);
    action(
        &mut app,
        EnvironmentSetupAction::DirectoryLoaded {
            request,
            result: Err("stale".into()),
        },
    );
    assert!(setup(&app).error.is_none());
    let request = setup(&app).browser.as_ref().unwrap().request;
    action(
        &mut app,
        EnvironmentSetupAction::DirectoryLoaded {
            request,
            result: Ok(EnvironmentDirectory {
                path: "/yocto".into(),
                children: vec!["/yocto/sub".into()],
                init_script: Some("/yocto/oe-init-build-env".into()),
                notice: None,
            }),
        },
    );
    action(&mut app, EnvironmentSetupAction::Select(isize::MAX));
    assert_eq!(setup(&app).browser.as_ref().unwrap().selection, 0);
    action(&mut app, EnvironmentSetupAction::ChooseDirectory);
    assert_eq!(setup(&app).values[0], "/yocto");
    assert_eq!(setup(&app).values[2], "/yocto/oe-init-build-env");
}
