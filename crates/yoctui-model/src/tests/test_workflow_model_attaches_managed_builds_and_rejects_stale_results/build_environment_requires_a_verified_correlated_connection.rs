use super::*;
#[test]
fn build_environment_requires_a_verified_correlated_connection() {
    let profile = BuildEnvironmentProfile {
        source_dir: "/workspace/poky".into(),
        build_dir: "/workspace/build".into(),
        init_script: "/workspace/poky/oe-init-build-env".into(),
    };
    let mut app = App::new_unconfigured(16, 4096);
    let request = BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    assert_eq!(update(&mut app, Action::Start(request.clone())), None);
    update(&mut app, Action::ConfigureBuildEnvironment(profile.clone()));
    let Some(Effect::SavedEnvironment(SavedEnvironmentRequest::PrepareProfile {
        generation,
        profile: requested,
    })) = update(&mut app, Action::BeginBuildEnvironmentVerification)
    else {
        panic!("daemon-aware verification")
    };
    assert_eq!(requested, profile);
    assert!(!app.build_environment.connected());
    let plan = SavedEnvironmentPlan {
        edited_profile: true,
        profile: profile.clone(),
        target: String::new(),
        machine: None,
        mode: SavedEnvironmentMode::Start,
    };
    let act = |app: &mut App, a| update(app, Action::SavedBuild(SavedBuildAction::Environment(a)));
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation: generation + 1,
            result: Ok(plan.clone()),
        },
    );
    assert!(app.active_dialog().is_none());
    act(
        &mut app,
        SavedEnvironmentAction::Prepared {
            generation,
            result: Ok(plan),
        },
    );
    assert!(matches!(
        act(&mut app, SavedEnvironmentAction::Confirm),
        Some(Effect::SavedEnvironment(
            SavedEnvironmentRequest::Load { .. }
        ))
    ));
    assert!(!app.build_environment.connected());
    act(
        &mut app,
        SavedEnvironmentAction::Finished {
            generation: generation + 1,
            result: Ok(profile.clone()),
        },
    );
    assert!(!app.build_environment.connected());
    act(
        &mut app,
        SavedEnvironmentAction::Finished {
            generation,
            result: Ok(profile.clone()),
        },
    );
    assert_eq!(
        app.build_environment,
        BuildEnvironmentState::Connected(profile)
    );
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
}
