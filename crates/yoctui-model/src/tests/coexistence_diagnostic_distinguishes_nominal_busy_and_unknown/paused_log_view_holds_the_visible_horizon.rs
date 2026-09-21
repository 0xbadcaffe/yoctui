use super::*;

#[test]
fn paused_log_view_holds_the_visible_horizon() {
    let mut app = App::new(10, 100);
    app.logs.insert(log("before pause"));
    let _ = update(&mut app, Action::ToggleLogFollow);
    app.logs.insert(log("after pause"));
    assert_eq!(app.logs.filtered().count(), 1);
    let _ = update(&mut app, Action::ToggleLogFollow);
    assert_eq!(app.logs.filtered().count(), 2);
}
