use super::*;

#[test]
fn build_environment_requires_a_verified_correlated_connection() {
    let profile = BuildEnvironmentProfile {
        source_dir: PathBuf::from("/workspace/poky"),
        build_dir: PathBuf::from("/workspace/build"),
        init_script: PathBuf::from("/workspace/poky/oe-init-build-env"),
    };
    let mut app = App::new_unconfigured(16, 4096);
    assert_eq!(app.screen, Screen::BuildEnvironment);
    assert_eq!(app.focus, FocusTarget::Navigator);
    let request = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    assert_eq!(update(&mut app, Action::Start(request.clone())), None);
    assert_eq!(
        app.notification.as_deref(),
        Some("Configure and verify a BitBake environment first")
    );

    assert_eq!(
        update(&mut app, Action::ConfigureBuildEnvironment(profile.clone())),
        None
    );
    let Some(Effect::VerifyBuildEnvironment { generation, .. }) =
        update(&mut app, Action::BeginBuildEnvironmentVerification)
    else {
        panic!("verification effect");
    };
    let _ = update(
        &mut app,
        Action::BuildEnvironmentVerified {
            generation: generation + 1,
        },
    );
    assert!(matches!(
        app.build_environment,
        BuildEnvironmentState::Verifying { .. }
    ));
    let _ = update(&mut app, Action::BuildEnvironmentVerified { generation });
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Connected(profile)
    );
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
}
