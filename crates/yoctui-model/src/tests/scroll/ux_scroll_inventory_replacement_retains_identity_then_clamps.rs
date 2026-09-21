use super::*;

#[test]
fn ux_scroll_inventory_replacement_retains_identity_then_clamps() {
    let before = ["alpha", "beta", "gamma"];
    let after = ["gamma", "beta", "delta"];
    let selected = reconcile_selected_identity(Some(&before[1]), 1, &after, |item| item);
    assert_eq!(selected, 1);
    let selected = reconcile_selected_identity(Some(&before[0]), 99, &after, |item| item);
    assert_eq!(selected, 2);
}
