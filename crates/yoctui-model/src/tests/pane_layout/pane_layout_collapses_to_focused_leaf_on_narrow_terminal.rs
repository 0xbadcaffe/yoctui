use super::*;

#[test]
fn pane_layout_collapses_to_focused_leaf_on_narrow_terminal() {
    let mut layout = PaneLayout::new(PaneId(1)).unwrap();
    layout.split(PaneId(1), SplitAxis::Horizontal).unwrap();
    assert_eq!(layout.visible_panes(20, 40), vec![PaneId(2)]);
    assert_eq!(layout.visible_panes(80, 40).len(), 2);
}
