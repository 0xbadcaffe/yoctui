use super::*;

#[test]
fn ux_onboarding_advances_only_with_exact_evidence_and_routes_through_typed_actions() {
    let mut app = App::new_unconfigured(32, 4096);
    app.onboarding.open = true;
    assert_eq!(update(&mut app, Action::AdvanceOnboarding), None);
    assert_eq!(app.onboarding.progress.current, OnboardingStep::Environment);

    app.build_environment = BuildEnvironmentState::Connected(crate::BuildEnvironmentProfile {
        source_dir: "/work/poky".into(),
        build_dir: "/work/build".into(),
        init_script: "/work/poky/oe-init-build-env".into(),
    });
    assert_eq!(
        update(&mut app, Action::AdvanceOnboarding),
        Some(crate::Effect::PersistOnboarding)
    );
    assert_eq!(app.onboarding.progress.current, OnboardingStep::Target);

    assert_eq!(
        onboarding_route(&app, OnboardingStep::FirstBuild),
        Action::OpenBuildOptions
    );
    assert_eq!(
        onboarding_route(&app, OnboardingStep::Terminal),
        Action::Open(Screen::TerminalSessions)
    );
    app.daemon.status = ClientReplicaStatus::Current;
    assert!(onboarding_completion_evidence(
        &app,
        OnboardingStep::Terminal
    ));
}
