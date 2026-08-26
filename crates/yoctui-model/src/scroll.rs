use std::ops::Range;

/// A closed navigation vocabulary shared by bounded collections and documents.
///
/// The viewport is presentation input. It never becomes inventory authority and
/// can therefore be replaced after every resize without losing selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollCommand {
    Rows(isize),
    Pages(isize),
    First,
    Last,
    Horizontal(isize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundedScroll {
    pub selection: usize,
    pub offset: usize,
    pub viewport: usize,
    pub total: usize,
}

impl BoundedScroll {
    pub fn new(selection: usize, offset: usize, viewport: usize, total: usize) -> Self {
        let mut state = Self {
            selection,
            offset,
            viewport,
            total,
        };
        state.reconcile(viewport, total);
        state
    }

    pub fn reconcile(&mut self, viewport: usize, total: usize) {
        self.viewport = viewport;
        self.total = total;
        if total == 0 {
            self.selection = 0;
            self.offset = 0;
            return;
        }
        self.selection = self.selection.min(total - 1);
        let visible = viewport.max(1).min(total);
        self.offset = self.offset.min(total.saturating_sub(visible));
        if self.selection < self.offset {
            self.offset = self.selection;
        } else if self.selection >= self.offset.saturating_add(visible) {
            self.offset = self.selection.saturating_add(1).saturating_sub(visible);
        }
    }

    pub fn apply(&mut self, command: ScrollCommand) {
        if self.total == 0 {
            self.selection = 0;
            self.offset = 0;
            return;
        }
        self.selection = match command {
            ScrollCommand::Rows(delta) => shifted_index(self.selection, delta, self.total),
            ScrollCommand::Pages(delta) => {
                let page = self.viewport.max(1);
                shifted_index(
                    self.selection,
                    delta.saturating_mul(page as isize),
                    self.total,
                )
            }
            ScrollCommand::First => 0,
            ScrollCommand::Last => self.total - 1,
            ScrollCommand::Horizontal(_) => self.selection,
        };
        self.reconcile(self.viewport, self.total);
    }

    pub fn visible_range(self) -> Range<usize> {
        if self.total == 0 || self.viewport == 0 {
            return 0..0;
        }
        self.offset..self.offset.saturating_add(self.viewport).min(self.total)
    }

    pub fn position(self) -> Option<(usize, usize)> {
        (self.total > 0).then_some((self.selection.min(self.total - 1) + 1, self.total))
    }

    pub fn range_label(self) -> String {
        if self.total == 0 {
            return "0/0".into();
        }
        let range = self.visible_range();
        if range.is_empty() {
            return format!("{}/{}", self.selection + 1, self.total);
        }
        format!("{}-{}/{}", range.start + 1, range.end, self.total)
    }
}

pub fn shifted_index(current: usize, delta: isize, total: usize) -> usize {
    if total == 0 {
        return 0;
    }
    if delta.is_negative() {
        current.saturating_sub(delta.unsigned_abs())
    } else {
        current.saturating_add(delta as usize).min(total - 1)
    }
}

/// Reconcile a replaced inventory by stable identity before falling back to a
/// bounded prior index. Callers remain the owners of the identity and rows.
pub fn reconcile_selected_identity<T, K: PartialEq>(
    previous_identity: Option<&K>,
    previous_index: usize,
    items: &[T],
    identity: impl Fn(&T) -> &K,
) -> usize {
    previous_identity
        .and_then(|wanted| items.iter().position(|item| identity(item) == wanted))
        .unwrap_or_else(|| previous_index.min(items.len().saturating_sub(1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

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

    #[test]
    fn ux_scroll_inventory_replacement_retains_identity_then_clamps() {
        let before = ["alpha", "beta", "gamma"];
        let after = ["gamma", "beta", "delta"];
        let selected = reconcile_selected_identity(Some(&before[1]), 1, &after, |item| item);
        assert_eq!(selected, 1);
        let selected = reconcile_selected_identity(Some(&before[0]), 99, &after, |item| item);
        assert_eq!(selected, 2);
    }

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
}
