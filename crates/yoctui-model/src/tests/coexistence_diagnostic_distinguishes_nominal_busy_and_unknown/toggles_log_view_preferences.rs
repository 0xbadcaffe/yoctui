use super::*;

#[test]
fn toggles_log_view_preferences() {
    let mut app = App::new(2, 10);
    let _ = update(&mut app, Action::ToggleLogFollow);
    let _ = update(&mut app, Action::ToggleLogWrap);
    assert!(!app.logs.follow);
    assert!(app.logs.wrap);
}
