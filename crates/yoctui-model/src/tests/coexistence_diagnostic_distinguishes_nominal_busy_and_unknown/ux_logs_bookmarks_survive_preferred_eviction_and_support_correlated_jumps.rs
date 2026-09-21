use super::*;

#[test]
fn ux_logs_bookmarks_survive_preferred_eviction_and_support_correlated_jumps() {
    let mut app = App::new(3, 1_000);
    for message in ["first", "second", "third"] {
        let _ = update(&mut app, Action::Log(log(message)));
    }
    app.logs.follow = false;
    app.logs.paused_len = Some(app.logs.entries.len());
    app.logs.selection = 0;
    let first_id = app.logs.selected().unwrap().id;
    let _ = update(&mut app, Action::ToggleSelectedLogBookmark);
    assert!(app.logs.is_bookmarked(first_id));

    let _ = update(&mut app, Action::Log(log("fourth")));
    assert!(app.logs.entries.iter().any(|entry| entry.id == first_id));
    assert!(
        !app.logs
            .entries
            .iter()
            .any(|entry| entry.message == "second")
    );

    app.logs.paused_len = None;
    let third_id = app
        .logs
        .entries
        .iter()
        .find(|entry| entry.message == "third")
        .unwrap()
        .id;
    assert!(app.logs.jump_to(third_id));
    let _ = update(&mut app, Action::ToggleSelectedLogBookmark);
    let _ = update(&mut app, Action::NextLogBookmark);
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(first_id));
    let _ = update(&mut app, Action::PreviousLogBookmark);
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(third_id));

    assert!(!app.logs.jump_to(u64::MAX));
    assert_eq!(app.logs.selected().map(|entry| entry.id), Some(third_id));
}
