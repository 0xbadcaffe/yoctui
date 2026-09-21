use super::*;

#[test]
fn ux_onboarding_progress_validation_rejects_future_and_conflicting_state() {
    let mut future = OnboardingProgress::default();
    future.schema_version += 1;
    assert!(future.validate().is_err());

    let mut conflict = OnboardingProgress::default();
    conflict.completed.insert(OnboardingStep::Environment);
    conflict.skipped.insert(OnboardingStep::Environment);
    assert!(conflict.validate().is_err());
}
