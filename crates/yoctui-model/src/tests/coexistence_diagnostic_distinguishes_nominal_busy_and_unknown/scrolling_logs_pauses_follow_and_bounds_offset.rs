use super::*;

#[test]
fn scrolling_logs_pauses_follow_and_bounds_offset() {
    let mut app = App::new(10, 1_000);
    let _ = update(&mut app, Action::Log(log("first")));
    let _ = update(&mut app, Action::Log(log("second")));
    let _ = update(&mut app, Action::ScrollLogs { delta: 9 });
    assert!(!app.logs.follow);
    assert_eq!(app.logs.scroll_offset, 1);
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("first")
    );
    let _ = update(&mut app, Action::ScrollLogs { delta: -9 });
    assert_eq!(app.logs.scroll_offset, 0);
    assert_eq!(
        app.logs.selected().map(|entry| entry.message.as_str()),
        Some("second")
    );
}
