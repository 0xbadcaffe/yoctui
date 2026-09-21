use super::*;

#[test]
fn ux_throbber_phase_is_reducer_owned_and_terminal_states_never_animate() {
    let fast = ActivityProjection::new(ActivityLifecycle::Running, 7, AnimationSpeed::Fast, false);
    assert_eq!(fast.phase, Some(1));
    let slow = ActivityProjection::new(ActivityLifecycle::Loading, 7, AnimationSpeed::Slow, false);
    assert_eq!(slow.phase, Some(2));
    let reduced = ActivityProjection::new(
        ActivityLifecycle::Waiting,
        u64::MAX,
        AnimationSpeed::Fast,
        true,
    );
    assert_eq!(reduced.phase, None);
    assert_eq!(reduced.text(), "waiting");
    for lifecycle in [
        ActivityLifecycle::Succeeded,
        ActivityLifecycle::Failed,
        ActivityLifecycle::Cancelled,
    ] {
        let terminal = ActivityProjection::new(lifecycle, u64::MAX, AnimationSpeed::Fast, false);
        assert_eq!(terminal.phase, None);
        assert_eq!(terminal.text(), lifecycle.word());
    }
}
