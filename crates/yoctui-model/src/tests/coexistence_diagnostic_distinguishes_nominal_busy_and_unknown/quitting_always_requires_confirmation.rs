use super::*;

#[test]
fn quitting_always_requires_confirmation() {
    let mut idle = App::new(2, 10);
    update(&mut idle, Action::Quit);
    assert!(matches!(
        idle.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
    assert!(!idle.should_quit);

    let mut a = App::new(2, 10);
    a.build.status = BuildStatus::Running;
    update(&mut a, Action::Quit);
    assert!(matches!(a.active_dialog(), Some(Dialog::QuitConfirmation)));
    assert!(!a.should_quit)
}
