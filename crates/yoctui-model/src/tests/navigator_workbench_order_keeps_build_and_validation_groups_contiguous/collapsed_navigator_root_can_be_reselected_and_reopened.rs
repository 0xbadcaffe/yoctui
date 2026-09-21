use super::*;

#[test]
fn collapsed_navigator_root_can_be_reselected_and_reopened() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = NAVIGATOR_GROUPS[2].start;
    let _ = update(&mut app, Action::CollapseNavigatorGroup);

    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    assert_eq!(app.navigator_group_index(), 3);
    let _ = update(&mut app, Action::SelectNavigator { delta: -1 });
    assert_eq!(app.navigator_selection, NAVIGATOR_GROUPS[2].start);
    assert_eq!(app.navigator_visual_row(), 11);

    let _ = update(&mut app, Action::ExpandNavigatorGroup);
    assert!(app.navigator_groups_expanded[2]);
    assert_eq!(app.screen, Screen::Dashboard, "reopening must not navigate");
}
