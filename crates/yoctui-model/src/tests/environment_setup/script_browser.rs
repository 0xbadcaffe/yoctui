use super::*;

fn open_script_browser(app: &mut App) -> u64 {
    let Some(Effect::ReadEnvironmentDirectory {
        request,
        include_files: true,
        path,
        ..
    }) = action(app, EnvironmentSetupAction::Browse)
    else {
        panic!("script scan missing")
    };
    assert_eq!(path, PathBuf::from("/yocto"));
    action(
        app,
        EnvironmentSetupAction::DirectoryLoaded {
            request,
            result: Ok(EnvironmentDirectory {
                path: "/yocto".into(),
                children: vec!["/yocto/nested".into()],
                files: vec![
                    "/yocto/custom setup".into(),
                    "/yocto/oe-init-build-env".into(),
                ],
                init_script: Some("/yocto/oe-init-build-env".into()),
                notice: None,
            }),
        },
    );
    request
}

#[test]
fn environment_setup_script_browser_selects_files_without_changing_other_paths() {
    for choose in [
        EnvironmentSetupAction::EnterDirectory,
        EnvironmentSetupAction::ChooseDirectory,
    ] {
        let mut app = App::new_unconfigured(20, 2000);
        action(&mut app, EnvironmentSetupAction::Open { browse: false });
        let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog_mut() else {
            unreachable!()
        };
        s.values = ["/yocto".into(), "/build".into(), String::new()];
        s.field = 2;
        open_script_browser(&mut app);
        action(&mut app, EnvironmentSetupAction::ChooseDirectory);
        assert!(
            setup(&app)
                .error
                .as_ref()
                .unwrap()
                .contains("Select a script file")
        );
        assert!(setup(&app).values[2].is_empty());
        action(&mut app, EnvironmentSetupAction::Select(isize::MAX));
        assert_eq!(setup(&app).browser.as_ref().unwrap().selection, 2);
        action(&mut app, EnvironmentSetupAction::Select(-1));
        action(&mut app, choose);
        assert_eq!(
            setup(&app).values,
            ["/yocto", "/build", "/yocto/custom setup"]
        );
        assert!(setup(&app).browser.is_none());
        assert!(setup(&app).error.is_none());
        // Cancelling browsing keeps the draft.
        let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog_mut() else {
            unreachable!()
        };
        s.values[2].clear();
        open_script_browser(&mut app);
        action(&mut app, EnvironmentSetupAction::Cancel);
        assert!(setup(&app).values[2].is_empty());
        assert!(matches!(
            app.build_environment,
            BuildEnvironmentState::Unconfigured
        ));
    }
}

#[test]
fn environment_setup_script_browser_opens_folders_and_restores_existing_selection() {
    let mut app = App::new_unconfigured(20, 2000);
    action(&mut app, EnvironmentSetupAction::Open { browse: false });
    let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog_mut() else {
        unreachable!()
    };
    s.values = ["/yocto".into(), "/build".into(), String::new()];
    s.field = 2;
    open_script_browser(&mut app);
    let effect = action(&mut app, EnvironmentSetupAction::EnterDirectory);
    assert!(
        matches!(effect, Some(Effect::ReadEnvironmentDirectory { path, include_files: true, initial: false, .. }) if path == std::path::Path::new("/yocto/nested"))
    );
    assert!(setup(&app).values[2].is_empty());
    action(&mut app, EnvironmentSetupAction::Cancel);
    let Some(Dialog::EnvironmentSetup(s)) = app.active_dialog_mut() else {
        unreachable!()
    };
    s.values[2] = "/yocto/custom setup".into();
    let Some(Effect::ReadEnvironmentDirectory { request, path, .. }) =
        action(&mut app, EnvironmentSetupAction::Browse)
    else {
        panic!("scan missing")
    };
    assert_eq!(path, PathBuf::from("/yocto/custom setup"));
    action(
        &mut app,
        EnvironmentSetupAction::DirectoryLoaded {
            request,
            result: Ok(EnvironmentDirectory {
                path: "/yocto".into(),
                children: vec!["/yocto/nested".into()],
                files: vec!["/yocto/custom setup".into()],
                init_script: None,
                notice: None,
            }),
        },
    );
    assert_eq!(setup(&app).browser.as_ref().unwrap().selection, 1);
    action(&mut app, EnvironmentSetupAction::Cancel);
    assert_eq!(setup(&app).values[2], "/yocto/custom setup");
}
