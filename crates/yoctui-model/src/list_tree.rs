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
    if value.len() <= limit {
        return value;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_list_tree_projection_bounds_duplicates_depth_height_and_unicode() {
        let mut rows = vec![ListTreeRow {
            id: "猫".into(),
            label: "レイヤー".into(),
            depth: usize::MAX,
            branch: true,
            expanded: true,
            height: usize::MAX,
        }];
        rows.push(rows[0].clone());
        rows.extend(
            (0..LIST_TREE_MAX_ROWS + 3)
                .map(|index| ListTreeRow::new(format!("id:{index}"), format!("row {index}"), 0)),
        );
        let tree = ListTreeProjection::new(rows);
        assert_eq!(tree.rows().len(), LIST_TREE_MAX_ROWS);
        assert_eq!(tree.limitations.duplicate_ids, 1);
        assert!(tree.limitations.omitted_rows > 0);
        assert_eq!(tree.rows()[0].depth, LIST_TREE_MAX_DEPTH);
        assert_eq!(tree.rows()[0].height, LIST_TREE_MAX_ROW_HEIGHT);
        assert!(list_tree_text(&tree.rows()[0], true, true).contains("▾ レイヤー"));
        assert!(list_tree_text(&tree.rows()[0], true, false).contains("- レイヤー"));
    }

    #[test]
    fn ux_list_tree_variable_height_window_keeps_selection_visible_and_bounded() {
        let window = variable_height_window([2, 4, 1, 8, 3], Some(3), 10);
        assert!(window.start <= 3 && window.end > 3);
        assert!(window.end <= 5);
        assert_eq!(window.total_height, 18);
        assert!(window.used_height <= 10);
        assert_eq!(variable_height_window([1, 2], Some(1), 0).end, 0);
    }
}
