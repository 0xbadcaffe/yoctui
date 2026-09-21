use super::*;

#[test]
fn animation_ticks_advance_unless_reduced_motion_is_enabled() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Tick);
    let _ = update(&mut app, Action::Tick);
    assert_eq!(app.animation_frame, 2);

    app.reduced_motion = true;
    let _ = update(&mut app, Action::Tick);
    assert_eq!(app.animation_frame, 2);
}
