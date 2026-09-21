use super::*;

#[test]
fn clone_activity_reducer_is_idempotent() {
    let mut app = App::new(10, 1024);
    for _ in 0..2 {
        update(
            &mut app,
            Action::SetBackgroundActivity {
                activity: BackgroundActivity::Cloning,
                active: true,
            },
        );
    }
    assert_eq!(app.background_activities.len(), 1);
    assert!(update(&mut app, Action::ConfirmBuildEnvironmentClone).is_none());
    update(
        &mut app,
        Action::SetBackgroundActivity {
            activity: BackgroundActivity::Cloning,
            active: false,
        },
    );
    assert!(app.background_activities.is_empty());
}
