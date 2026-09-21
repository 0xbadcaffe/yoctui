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
#[path = "tests/scroll/mod.rs"]
mod tests;
