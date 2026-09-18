use super::*;

#[test]
fn ux_onboarding_first_run_resume_dismissal_and_validation_are_persistent_and_safe() {
    let directory = std::env::temp_dir().join(format!("yoctui-onboarding-{}", std::process::id()));
    if directory.exists() {
        fs::remove_dir_all(&directory).unwrap();
    }
    fs::create_dir(&directory).unwrap();
    let path = directory.join("session.toml");
    let mut session = Session::default();
    let mut app = App::new_unconfigured(8, 1024);

    install_session_onboarding(&session, &mut app).unwrap();
    assert!(
        app.onboarding.open,
        "missing state opens the first-run guide"
    );
    assert_eq!(app.build.status, BuildStatus::Idle);
    assert!(app.daemon.pty_sessions.is_empty());

    app.onboarding
        .progress
        .skipped
        .insert(yoctui_model::OnboardingStep::Environment);
    app.onboarding.progress.current = yoctui_model::OnboardingStep::Target;
    assert_eq!(
        update(&mut app, Action::DismissOnboarding),
        Some(Effect::PersistOnboarding)
    );
    persist_onboarding(Some(&path), &mut session, &app).unwrap();

    let loaded = read_session(Some(&path)).unwrap();
    let mut restored = App::new_unconfigured(8, 1024);
    install_session_onboarding(&loaded, &mut restored).unwrap();
    assert!(
        !restored.onboarding.open,
        "persisted state does not auto-reopen"
    );
    assert_eq!(
        restored.onboarding.progress.current,
        yoctui_model::OnboardingStep::Target
    );
    assert!(restored.onboarding.progress.dismissed);

    let before = fs::read(&path).unwrap();
    restored.onboarding.progress.schema_version += 1;
    assert!(persist_onboarding(Some(&path), &mut session, &restored).is_err());
    assert_eq!(
        fs::read(&path).unwrap(),
        before,
        "invalid state is fail-closed"
    );

    fs::remove_file(path).unwrap();
    fs::remove_dir(directory).unwrap();
}
