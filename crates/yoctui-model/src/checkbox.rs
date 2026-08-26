pub const CHECKBOX_MAX_BATCH_ROWS: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxValue {
    Unchecked,
    Checked,
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxState {
    pub id: String,
    pub label: String,
    pub value: CheckboxValue,
    pub enabled: bool,
    pub focused: bool,
    pub disabled_reason: Option<String>,
}

impl CheckboxState {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            value: CheckboxValue::Unchecked,
            enabled: true,
            focused: false,
            disabled_reason: None,
        }
    }

    pub fn set_disabled(&mut self, reason: impl Into<String>) {
        self.enabled = false;
        self.disabled_reason = Some(reason.into());
    }

    pub fn toggle(&mut self) -> bool {
        if !self.enabled {
            return false;
        }
        self.value = match self.value {
            CheckboxValue::Unchecked | CheckboxValue::Indeterminate => CheckboxValue::Checked,
            CheckboxValue::Checked => CheckboxValue::Unchecked,
        };
        true
    }

    pub fn selected(&self) -> bool {
        self.value == CheckboxValue::Checked
    }

    pub fn marker(&self, unicode: bool) -> &'static str {
        match (unicode, self.value) {
            (true, CheckboxValue::Unchecked) => "☐",
            (true, CheckboxValue::Checked) => "☑",
            (true, CheckboxValue::Indeterminate) => "⊟",
            (false, CheckboxValue::Unchecked) => "[ ]",
            (false, CheckboxValue::Checked) => "[x]",
            (false, CheckboxValue::Indeterminate) => "[-]",
        }
    }

    pub fn semantic_state(&self) -> &'static str {
        if !self.enabled {
            "disabled"
        } else {
            match self.value {
                CheckboxValue::Unchecked => "unchecked",
                CheckboxValue::Checked => "checked",
                CheckboxValue::Indeterminate => "indeterminate",
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CheckboxBatch {
    rows: Vec<CheckboxState>,
    cursor: usize,
}

impl CheckboxBatch {
    pub fn new(rows: impl IntoIterator<Item = CheckboxState>) -> Self {
        let rows = rows.into_iter().take(CHECKBOX_MAX_BATCH_ROWS).collect();
        Self { rows, cursor: 0 }
    }

    pub fn rows(&self) -> &[CheckboxState] {
        &self.rows
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn move_cursor(&mut self, delta: isize) {
        self.cursor = self
            .cursor
            .saturating_add_signed(delta)
            .min(self.rows.len().saturating_sub(1));
        for (index, row) in self.rows.iter_mut().enumerate() {
            row.focused = index == self.cursor;
        }
    }

    pub fn toggle_focused(&mut self) -> bool {
        self.rows
            .get_mut(self.cursor)
            .is_some_and(CheckboxState::toggle)
    }

    pub fn selected_ids(&self) -> Vec<&str> {
        self.rows
            .iter()
            .filter(|row| row.enabled && row.selected())
            .map(|row| row.id.as_str())
            .collect()
    }

    pub fn preview(&self, destructive: bool) -> CheckboxBatchPreview {
        CheckboxBatchPreview {
            targets: self.selected_ids().into_iter().map(str::to_owned).collect(),
            destructive,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckboxBatchPreview {
    pub targets: Vec<String>,
    pub destructive: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_checkbox_states_markers_focus_and_disabled_reason_are_explicit() {
        let mut row = CheckboxState::new("pkg:a", "Package A");
        assert_eq!((row.marker(true), row.marker(false)), ("☐", "[ ]"));
        assert!(row.toggle());
        assert_eq!((row.marker(true), row.semantic_state()), ("☑", "checked"));
        row.value = CheckboxValue::Indeterminate;
        assert_eq!(
            (row.marker(false), row.semantic_state()),
            ("[-]", "indeterminate")
        );
        row.set_disabled("not available for this image");
        assert!(!row.toggle());
        assert_eq!(row.semantic_state(), "disabled");
    }

    #[test]
    fn ux_checkbox_batch_is_bounded_and_preview_never_executes() {
        let mut batch =
            CheckboxBatch::new((0..CHECKBOX_MAX_BATCH_ROWS + 20).map(|index| {
                CheckboxState::new(format!("pkg:{index}"), format!("Package {index}"))
            }));
        assert_eq!(batch.rows().len(), CHECKBOX_MAX_BATCH_ROWS);
        assert!(batch.toggle_focused());
        batch.move_cursor(1);
        assert!(batch.toggle_focused());
        let preview = batch.preview(true);
        assert_eq!(preview.targets, ["pkg:0", "pkg:1"]);
        assert!(preview.destructive);
    }
}
