use super::*;

#[test]
fn navigator_groups_collapse_without_exposing_hidden_destinations() {
    let mut app = App::new(10, 1_000);
    app.navigator_selection = 10;
    assert_eq!(app.navigator_group_index(), 2);
    assert_eq!(app.navigator_visual_row(), 13);

    let _ = update(&mut app, Action::CollapseNavigatorGroup);
    assert!(!app.navigator_groups_expanded[2]);
    assert_eq!(app.navigator_visual_row(), 12);
    assert_eq!(app.navigator_group_at_visual_row(12), Some(2));
    assert_eq!(app.navigator_selection_at_visual_row(12), None);

    let _ = update(&mut app, Action::SelectNavigator { delta: 1 });
    assert_eq!(app.navigator_selection, 15);
    let _ = update(&mut app, Action::SelectNavigatorAt { index: 11 });
    assert_eq!(
        app.navigator_selection, 15,
        "hidden rows cannot be selected"
    );

    app.navigator_selection = 10;
    let _ = update(&mut app, Action::ActivateNavigator);
    assert!(app.navigator_groups_expanded[2]);
    assert_eq!(app.screen, Screen::Dashboard, "expansion does not navigate");
}
