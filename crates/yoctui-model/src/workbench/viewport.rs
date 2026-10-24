use super::*;
pub fn workbench_gutter_width(lines: usize) -> usize {
    lines.to_string().len() + 4
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkbenchViewport {
    pub top: usize,
    pub left: usize,
    pub rows: usize,
    pub columns: usize,
    pub follow_cursor: bool,
}

pub fn workbench_visual_column(text: &str, column: usize) -> usize {
    text.chars().take(column).fold(0, |cells, c| {
        cells
            + if c == '\t' {
                WORKBENCH_TAB_WIDTH - cells % WORKBENCH_TAB_WIDTH
            } else {
                unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
            }
    })
}

pub fn workbench_source_column(text: &str, visual: usize) -> usize {
    let mut cells = 0;
    for (column, c) in text.chars().enumerate() {
        let next = cells
            + if c == '\t' {
                WORKBENCH_TAB_WIDTH - cells % WORKBENCH_TAB_WIDTH
            } else {
                unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
            };
        if next > visual {
            return column;
        }
        cells = next;
    }
    text.chars().count()
}

impl WorkbenchTab {
    pub fn reveal_cursor(&mut self) {
        let p = self.document.position();
        let rows = self.viewport.rows.max(1);
        if p.line < self.viewport.top {
            self.viewport.top = p.line;
        }
        if p.line >= self.viewport.top + rows {
            self.viewport.top = p.line + 1 - rows;
        }
        let line = self
            .document
            .text
            .split('\n')
            .nth(p.line)
            .unwrap_or_default();
        let column = workbench_visual_column(line, p.column);
        let columns = self.viewport.columns.max(1);
        if column < self.viewport.left {
            self.viewport.left = column;
        }
        if column >= self.viewport.left + columns {
            self.viewport.left = column + 1 - columns;
        }
        self.viewport.follow_cursor = true;
    }

    pub fn relocate_breakpoints(&mut self, before: &str) {
        if self.breakpoints.is_empty() || before == self.document.text {
            return;
        }
        let after = &self.document.text;
        let prefix = before
            .bytes()
            .zip(after.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        let suffix = before.as_bytes()[prefix..]
            .iter()
            .rev()
            .zip(after.as_bytes()[prefix..].iter().rev())
            .take_while(|(a, b)| a == b)
            .count();
        let line = |text: &str, end: usize| {
            text.as_bytes()[..end]
                .iter()
                .filter(|b| **b == b'\n')
                .count()
                + 1
        };
        let first = line(before, prefix);
        let old_end = line(before, before.len() - suffix);
        let new_end = line(after, after.len() - suffix);
        let old_end_byte = before.len() - suffix;
        let end_at_line_start = old_end_byte == 0 || before.as_bytes()[old_end_byte - 1] == b'\n';
        let last = self.document.line_count();
        self.breakpoints = self
            .breakpoints
            .iter()
            .map(|&n| {
                if n < first {
                    n
                } else if n > old_end || (n == old_end && end_at_line_start) {
                    n.saturating_add_signed(new_end as isize - old_end as isize)
                } else {
                    n.min(new_end)
                }
                .min(last)
            })
            .collect();
    }
}
