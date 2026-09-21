use super::*;

#[test]
fn ux_onboarding_projection_distinguishes_current_blocked_skipped_stale_and_unavailable() {
    let mut app = App::new_unconfigured(32, 4096);
    app.onboarding.open = true;
    let projection = app.onboarding_projection();
    assert_eq!(projection.rows[0].status, OnboardingStepStatus::Current);
    assert_eq!(projection.rows[1].status, OnboardingStepStatus::Blocked);

    app.onboarding
        .progress
        .completed
        .insert(OnboardingStep::Environment);
    app.onboarding
        .progress
        .completed
        .insert(OnboardingStep::Target);
    let projection = app.onboarding_projection();
    assert_eq!(projection.rows[0].status, OnboardingStepStatus::Stale);
    assert_eq!(projection.rows[1].status, OnboardingStepStatus::Stale);
    assert!(!projection.finished);

    app.onboarding.progress.completed.clear();
    app.onboarding
        .progress
        .skipped
        .insert(OnboardingStep::Environment);
    app.onboarding.progress.current = OnboardingStep::Target;
    app.onboarding.selected = OnboardingStep::Target;
    let projection = app.onboarding_projection();
    assert_eq!(projection.rows[0].status, OnboardingStepStatus::Skipped);
    assert_eq!(projection.rows[1].status, OnboardingStepStatus::Unavailable);
}
