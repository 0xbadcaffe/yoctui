use super::*;

#[test]
fn pane_layout_rejects_invalid_focus() {
    let layout = PaneLayout::new(PaneId(9)).unwrap();
    let mut invalid = layout;
    invalid.focused = PaneId(99);
    assert!(invalid.validate().is_err());
}
