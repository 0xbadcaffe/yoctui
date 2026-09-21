use super::*;

#[test]
fn ux_accessibility_reduced_motion_keeps_every_activity_lifecycle_textual_and_stable() {
    for lifecycle in [
        ActivityLifecycle::Loading,
        ActivityLifecycle::Running,
        ActivityLifecycle::Waiting,
        ActivityLifecycle::Succeeded,
        ActivityLifecycle::Failed,
        ActivityLifecycle::Cancelled,
    ] {
        let first = ActivityProjection::new(lifecycle, 0, AnimationSpeed::Fast, true);
        let later = ActivityProjection::new(lifecycle, u64::MAX, AnimationSpeed::Slow, true);
        assert_eq!(first, later);
        assert_eq!(first.phase, None);
        assert_eq!(first.text(), lifecycle.word());
    }
}
