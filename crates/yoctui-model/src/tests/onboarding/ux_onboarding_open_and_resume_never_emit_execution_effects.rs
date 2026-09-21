use super::*;

#[test]
fn ux_onboarding_open_and_resume_never_emit_execution_effects() {
    let mut app = App::new_unconfigured(32, 4096);
    assert_eq!(update(&mut app, Action::OpenOnboarding), None);
    assert!(app.onboarding.open);
    assert_eq!(app.build.status, BuildStatus::Idle);
    assert!(app.daemon.pty_sessions.is_empty());

    let effect = update(&mut app, Action::DismissOnboarding);
    assert_eq!(effect, Some(crate::Effect::PersistOnboarding));
    assert!(!app.onboarding.open);
    assert!(app.onboarding.progress.dismissed);
}
