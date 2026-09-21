use super::*;

#[test]
fn ux_scroll_rows_pages_edges_resize_and_text_are_one_bounded_contract() {
    let mut state = BoundedScroll::new(4, 2, 3, 10);
    assert_eq!(state.visible_range(), 2..5);
    assert_eq!(state.range_label(), "3-5/10");

    state.apply(ScrollCommand::Pages(1));
    assert_eq!((state.selection, state.offset), (7, 5));
    state.apply(ScrollCommand::Last);
    assert_eq!((state.selection, state.offset), (9, 7));
    state.apply(ScrollCommand::First);
    assert_eq!((state.selection, state.offset), (0, 0));

    state.reconcile(1, 2);
    assert_eq!(state.visible_range(), 0..1);
    state.reconcile(20, 1);
    assert_eq!(state.visible_range(), 0..1);
    state.reconcile(0, 0);
    assert_eq!(state.range_label(), "0/0");
}
