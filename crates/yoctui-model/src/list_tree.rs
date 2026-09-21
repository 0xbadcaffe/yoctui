use std::collections::HashSet;

pub const LIST_TREE_MAX_ROWS: usize = 8_192;
pub const LIST_TREE_MAX_DEPTH: usize = 64;
pub const LIST_TREE_MAX_ROW_HEIGHT: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListTreeRow {
    pub id: String,
    pub label: String,
    pub depth: usize,
    pub branch: bool,
    pub expanded: bool,
    pub height: usize,
}

impl ListTreeRow {
    pub fn new(id: impl Into<String>, label: impl Into<String>, depth: usize) -> Self {
        Self {
            id: bounded_text(id.into(), 4_096),
            label: bounded_text(label.into(), 16_384),
            depth: depth.min(LIST_TREE_MAX_DEPTH),
            branch: false,
            expanded: false,
            height: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListTreeLimitations {
    pub duplicate_ids: usize,
    pub omitted_rows: usize,
    pub clamped_depths: usize,
    pub clamped_heights: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ListTreeProjection {
    rows: Vec<ListTreeRow>,
    pub limitations: ListTreeLimitations,
}

impl ListTreeProjection {
    pub fn new(rows: impl IntoIterator<Item = ListTreeRow>) -> Self {
        let mut projection = Self::default();
        let mut ids = HashSet::new();
        for mut row in rows {
            if !ids.insert(row.id.clone()) {
                projection.limitations.duplicate_ids += 1;
                continue;
            }
            if projection.rows.len() == LIST_TREE_MAX_ROWS {
                projection.limitations.omitted_rows += 1;
                continue;
            }
            if row.depth > LIST_TREE_MAX_DEPTH {
                row.depth = LIST_TREE_MAX_DEPTH;
                projection.limitations.clamped_depths += 1;
            }
            if row.height == 0 || row.height > LIST_TREE_MAX_ROW_HEIGHT {
                row.height = row.height.clamp(1, LIST_TREE_MAX_ROW_HEIGHT);
                projection.limitations.clamped_heights += 1;
            }
            projection.rows.push(row);
        }
        projection
    }

    pub fn rows(&self) -> &[ListTreeRow] {
        &self.rows
    }

    pub fn selected_index(&self, id: &str) -> Option<usize> {
        self.rows.iter().position(|row| row.id == id)
    }

    pub fn window(&self, selected: Option<usize>, viewport_height: usize) -> ListTreeWindow {
        variable_height_window(
            self.rows.iter().map(|row| row.height),
            selected,
            viewport_height,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ListTreeWindow {
    pub start: usize,
    pub end: usize,
    pub used_height: usize,
    pub total_height: usize,
}

pub fn variable_height_window(
    heights: impl IntoIterator<Item = usize>,
    selected: Option<usize>,
    viewport_height: usize,
) -> ListTreeWindow {
    let heights = heights
        .into_iter()
        .take(LIST_TREE_MAX_ROWS)
        .map(|height| height.clamp(1, LIST_TREE_MAX_ROW_HEIGHT))
        .collect::<Vec<_>>();
    let total_height = heights.iter().sum();
    if heights.is_empty() || viewport_height == 0 {
        return ListTreeWindow {
            total_height,
            ..ListTreeWindow::default()
        };
    }
    let selected = selected.unwrap_or(0).min(heights.len() - 1);
    let mut start = selected;
    let mut used = heights[selected];
    while start > 0 && used.saturating_add(heights[start - 1]) <= viewport_height {
        start -= 1;
        used += heights[start];
    }
    let mut end = selected + 1;
    while end < heights.len() && used.saturating_add(heights[end]) <= viewport_height {
        used += heights[end];
        end += 1;
    }
    while end < heights.len() && used < viewport_height {
        used = used.saturating_add(heights[end]);
        end += 1;
    }
    ListTreeWindow {
        start,
        end,
        used_height: used.min(viewport_height.max(heights[selected])),
        total_height,
    }
}

pub fn list_tree_text(row: &ListTreeRow, selected: bool, unicode: bool) -> String {
    let focus = if selected { ">" } else { " " };
    let branch = match (unicode, row.branch, row.expanded) {
        (_, false, _) => " ",
        (true, true, true) => "▾",
        (true, true, false) => "▸",
        (false, true, true) => "-",
        (false, true, false) => "+",
    };
    format!("{focus} {}{branch} {}", "  ".repeat(row.depth), row.label)
}

fn bounded_text(mut value: String, limit: usize) -> String {
    yoctui_utils::truncate_utf8(&mut value, limit);
    value
}

#[cfg(test)]
#[path = "tests/list_tree/mod.rs"]
mod tests;
