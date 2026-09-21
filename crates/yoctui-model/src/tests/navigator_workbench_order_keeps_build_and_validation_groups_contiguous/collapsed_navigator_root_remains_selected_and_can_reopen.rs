use super::*;

#[test]
fn collapsed_navigator_root_remains_selected_and_can_reopen() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = NAVIGATOR_GROUPS[4].start + 3;

    let _ = update(&mut app, Action::ToggleNavigatorGroup { group: 1 });
    assert!(!app.navigator_groups_expanded[1]);
    assert_eq!(app.navigator_selection, NAVIGATOR_GROUPS[1].start);
    assert_eq!(app.navigator_group_index(), 1);

    let _ = update(&mut app, Action::ExpandNavigatorGroup);
    assert!(app.navigator_groups_expanded[1]);

    let _ = update(&mut app, Action::ToggleNavigatorGroup { group: 1 });
    assert!(!app.navigator_groups_expanded[1]);
    let _ = update(&mut app, Action::ActivateNavigator);
    assert!(app.navigator_groups_expanded[1]);
    assert_eq!(app.screen, Screen::Dashboard, "reopening does not navigate");
}
