impl TextAreaState {
    pub fn new(mut text: String) -> Self {
        truncate_utf8(&mut text, TEXTAREA_MAX_BYTES);
        let cursor = text.len();
        let revision = TextAreaRevision::of(&text);
        let base_text = text.clone();
        let diff_base_text = text.clone();
        Self {
            text,
            cursor,
            selection: None,
            editing: false,
            advanced: Box::new(TextAreaAdvancedState {
                base_text,
                diff_base_text,
                layout: TextAreaLayout::default(),
                search: TextAreaSearchState::default(),
                validation: Vec::new(),
                save: TextAreaSaveState::Clean { revision },
                mode: TextAreaMode::Normal,
                visual_anchor: None,
                preferred_column: None,
                clipboard: String::new(),
                undo: VecDeque::new(),
                redo: VecDeque::new(),
            }),
        }
    }

    pub fn layout(&self) -> &TextAreaLayout {
        &self.advanced.layout
    }

    pub fn layout_mut(&mut self) -> &mut TextAreaLayout {
        &mut self.advanced.layout
    }

    pub fn search_state(&self) -> &TextAreaSearchState {
        &self.advanced.search
    }

    pub fn validation(&self) -> &[TextAreaValidationSpan] {
        &self.advanced.validation
    }

    pub fn save_state(&self) -> &TextAreaSaveState {
        &self.advanced.save
    }

    pub fn base_revision(&self) -> TextAreaRevision {
        TextAreaRevision::of(&self.advanced.base_text)
    }

    pub fn is_modified(&self) -> bool {
        self.advanced.base_text != self.text
    }

    pub fn has_visual_diff(&self) -> bool {
        self.advanced.diff_base_text != self.text
    }

    pub fn mode(&self) -> TextAreaMode {
        if self.editing {
            TextAreaMode::Insert
        } else {
            self.advanced.mode
        }
    }

    pub fn set_mode(&mut self, mode: TextAreaMode) {
        self.advanced.mode = mode;
        self.editing = mode == TextAreaMode::Insert;
        match mode {
            TextAreaMode::Visual => {
                self.advanced.visual_anchor = Some(self.cursor);
                self.selection = Some((self.cursor, self.cursor));
            }
            TextAreaMode::Normal => {
                self.advanced.visual_anchor = None;
                self.selection = None;
            }
            TextAreaMode::Insert => {
                self.advanced.visual_anchor = None;
            }
        }
    }

    pub fn toggle_insert(&mut self) {
        let mode = if self.mode() == TextAreaMode::Insert {
            TextAreaMode::Normal
        } else {
            TextAreaMode::Insert
        };
        self.set_mode(mode);
    }

    pub fn position(&self) -> TextAreaPosition {
        position_at(&self.text, self.cursor)
    }

    pub fn line_count(&self) -> usize {
        self.text.bytes().filter(|byte| *byte == b'\n').count() + 1
    }

    pub fn history_lengths(&self) -> (usize, usize) {
        (self.advanced.undo.len(), self.advanced.redo.len())
    }

    pub fn select_range(&mut self, start: usize, end: usize) {
        let start = clamp_boundary(&self.text, start);
        let end = clamp_boundary(&self.text, end);
        self.selection = Some((start.min(end), start.max(end)));
        self.cursor = end;
        self.advanced.visual_anchor = Some(start);
        self.advanced.preferred_column = None;
    }

    pub fn select_position(&mut self, line: usize, column: usize, extend: bool) {
        let target = offset_for_position(&self.text, line.min(self.line_count() - 1), column);
        if extend {
            let anchor = self
                .advanced
                .visual_anchor
                .or_else(|| self.selection.map(|(start, _)| start))
                .unwrap_or(self.cursor);
            self.advanced.mode = TextAreaMode::Visual;
            self.editing = false;
            self.advanced.visual_anchor = Some(anchor);
            self.cursor = target;
            self.selection = Some((anchor.min(target), anchor.max(target)));
        } else {
            self.cursor = target;
            self.selection = None;
            self.advanced.visual_anchor = Some(target);
            self.advanced.preferred_column = None;
        }
    }

    pub fn selected_text(&self) -> Option<&str> {
        self.selection.map(|(start, end)| &self.text[start..end])
    }

}
