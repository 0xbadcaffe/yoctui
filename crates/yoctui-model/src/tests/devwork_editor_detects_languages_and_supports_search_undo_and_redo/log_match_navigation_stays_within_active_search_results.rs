use super::*;

#[test]
fn log_match_navigation_stays_within_active_search_results() {
    let mut app = App::new(10, 1_000);
    app.logs.insert(log("alpha match"));
    app.logs.insert(log("not relevant"));
    app.logs.insert(log("beta match"));
    app.logs.query = "match".into();

    let _ = update(&mut app, Action::NextLogMatch);
    assert_eq!(app.logs.selection, 1);
    assert_eq!(app.logs.match_position(), Some((2, 2)));
    assert!(!app.logs.follow);

    let _ = update(&mut app, Action::NextLogMatch);
    assert_eq!(app.logs.selection, 1);
    let _ = update(&mut app, Action::PreviousLogMatch);
    assert_eq!(app.logs.selection, 0);
    assert_eq!(app.logs.scroll_offset, 1);
}
