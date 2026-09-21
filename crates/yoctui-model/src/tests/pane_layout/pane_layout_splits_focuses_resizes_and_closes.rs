use super::*;

#[test]
fn pane_layout_splits_focuses_resizes_and_closes() {
    let mut layout = PaneLayout::new(PaneId(1)).unwrap();
    let second = layout.split(PaneId(1), SplitAxis::Vertical).unwrap();
    assert_eq!(layout.focused, second);
    assert!(layout.resize(second, 100).is_ok());
    layout.focus(PaneId(1)).unwrap();
    layout.close(PaneId(1)).unwrap();
    assert_eq!(layout.pane_ids(), vec![second]);
    assert!(layout.validate().is_ok());
}
