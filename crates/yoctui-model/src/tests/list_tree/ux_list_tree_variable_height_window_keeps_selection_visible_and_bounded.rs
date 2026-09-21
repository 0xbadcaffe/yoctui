use super::*;

#[test]
fn ux_list_tree_variable_height_window_keeps_selection_visible_and_bounded() {
    let window = variable_height_window([2, 4, 1, 8, 3], Some(3), 10);
    assert!(window.start <= 3 && window.end > 3);
    assert!(window.end <= 5);
    assert_eq!(window.total_height, 18);
    assert!(window.used_height <= 10);
    assert_eq!(variable_height_window([1, 2], Some(1), 0).end, 0);
}
