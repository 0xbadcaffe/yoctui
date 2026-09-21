impl TextAreaState {
    pub fn insert(&mut self, value: &str) {
        let _ = self.try_insert(value);
    }

    pub fn try_insert(&mut self, value: &str) -> Result<(), TextAreaError> {
        let replaced = self.selection.map_or(0, |(start, end)| end - start);
        if self.text.len().saturating_sub(replaced) + value.len() > TEXTAREA_MAX_BYTES {
            return Err(TextAreaError::TextLimit {
                limit: TEXTAREA_MAX_BYTES,
            });
        }
        if value.is_empty() && replaced == 0 {
            return Ok(());
        }
        self.remember();
        if let Some((start, end)) = self.selection.take() {
            self.text.replace_range(start..end, value);
            self.cursor = start + value.len();
        } else {
            self.text.insert_str(self.cursor, value);
            self.cursor += value.len();
        }
        self.after_edit();
        Ok(())
    }

    pub fn paste_text(
        &mut self,
        value: &str,
        _source: TextAreaPasteSource,
    ) -> Result<(), TextAreaError> {
        if value.len() > TEXTAREA_MAX_PASTE_BYTES {
            return Err(TextAreaError::PasteLimit {
                limit: TEXTAREA_MAX_PASTE_BYTES,
            });
        }
        self.try_insert(value)
    }

    pub fn set_clipboard(&mut self, value: String) {
        self.advanced.clipboard = bounded_utf8(value, TEXTAREA_MAX_PASTE_BYTES);
    }

    pub fn paste_internal_clipboard(&mut self) -> Result<(), TextAreaError> {
        let value = self.advanced.clipboard.clone();
        self.paste_text(&value, TextAreaPasteSource::Clipboard)
    }

    pub fn paste(&mut self) {
        let _ = self.paste_internal_clipboard();
    }

    pub fn backspace(&mut self) {
        if self.delete_selection() {
            return;
        }
        if self.cursor == 0 {
            return;
        }
        let previous = previous_boundary(&self.text, self.cursor);
        self.remember();
        self.text.replace_range(previous..self.cursor, "");
        self.cursor = previous;
        self.after_edit();
    }

    pub fn delete_forward(&mut self) {
        if self.delete_selection() || self.cursor == self.text.len() {
            return;
        }
        let next = next_boundary(&self.text, self.cursor);
        self.remember();
        self.text.replace_range(self.cursor..next, "");
        self.after_edit();
    }

    fn delete_selection(&mut self) -> bool {
        let Some((start, end)) = self.selection.filter(|(start, end)| start < end) else {
            return false;
        };
        self.remember();
        self.text.replace_range(start..end, "");
        self.cursor = start;
        self.selection = None;
        self.after_edit();
        true
    }

    pub fn move_cursor(&mut self, motion: TextAreaMotion) {
        let old = self.cursor;
        self.cursor = match motion {
            TextAreaMotion::Left => previous_boundary(&self.text, self.cursor),
            TextAreaMotion::Right => next_boundary(&self.text, self.cursor),
            TextAreaMotion::WordLeft => word_left(&self.text, self.cursor),
            TextAreaMotion::WordRight => word_right(&self.text, self.cursor),
            TextAreaMotion::Up => self.vertical_cursor(-1),
            TextAreaMotion::Down => self.vertical_cursor(1),
            TextAreaMotion::LineStart => line_start(&self.text, self.cursor),
            TextAreaMotion::LineEnd => line_end(&self.text, self.cursor),
            TextAreaMotion::PageUp => {
                self.vertical_cursor(-(self.advanced.layout.viewport_rows as isize))
            }
            TextAreaMotion::PageDown => {
                self.vertical_cursor(self.advanced.layout.viewport_rows as isize)
            }
            TextAreaMotion::DocumentStart => 0,
            TextAreaMotion::DocumentEnd => self.text.len(),
        };
        if !matches!(
            motion,
            TextAreaMotion::Up
                | TextAreaMotion::Down
                | TextAreaMotion::PageUp
                | TextAreaMotion::PageDown
        ) {
            self.advanced.preferred_column = None;
        }
        if self.mode() == TextAreaMode::Visual {
            let anchor = clamp_boundary(&self.text, self.advanced.visual_anchor.unwrap_or(old));
            self.advanced.visual_anchor = Some(anchor);
            self.selection = Some((anchor.min(self.cursor), anchor.max(self.cursor)));
        } else {
            self.selection = None;
        }
    }

    pub fn left(&mut self) {
        self.move_cursor(TextAreaMotion::Left);
    }
    pub fn right(&mut self) {
        self.move_cursor(TextAreaMotion::Right);
    }
    pub fn up(&mut self) {
        self.move_cursor(TextAreaMotion::Up);
    }
    pub fn down(&mut self) {
        self.move_cursor(TextAreaMotion::Down);
    }
    pub fn home(&mut self) {
        self.move_cursor(TextAreaMotion::LineStart);
    }
    pub fn end(&mut self) {
        self.move_cursor(TextAreaMotion::LineEnd);
    }

    fn vertical_cursor(&mut self, delta: isize) -> usize {
        let position = self.position();
        let column = *self
            .advanced
            .preferred_column
            .get_or_insert(position.column);
        let target = position
            .line
            .saturating_add_signed(delta)
            .min(self.line_count() - 1);
        offset_for_position(&self.text, target, column)
    }

}
