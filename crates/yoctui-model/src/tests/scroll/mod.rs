use super::*;
use proptest::prelude::*;

mod ux_scroll_rows_pages_edges_resize_and_text_are_one_bounded_contract;

mod ux_scroll_inventory_replacement_retains_identity_then_clamps;

proptest! {
    #[test]
    fn ux_scroll_property_never_escapes_retained_bounds(
        total in 0usize..10_000,
        viewport in 0usize..1_000,
        selection in any::<usize>(),
        offset in any::<usize>(),
        commands in prop::collection::vec((-10_000isize..10_000, any::<bool>()), 0..100),
    ) {
        let mut state = BoundedScroll::new(selection, offset, viewport, total);
        for (delta, page) in commands {
            state.apply(if page { ScrollCommand::Pages(delta) } else { ScrollCommand::Rows(delta) });
            prop_assert!(state.total == 0 || state.selection < state.total);
            prop_assert!(state.offset <= state.total.saturating_sub(state.viewport.min(state.total)));
            let range = state.visible_range();
            prop_assert!(range.start <= range.end);
            prop_assert!(range.end <= state.total);
            if state.total > 0 && state.viewport > 0 {
                prop_assert!(range.contains(&state.selection));
            }
        }
    }
}
