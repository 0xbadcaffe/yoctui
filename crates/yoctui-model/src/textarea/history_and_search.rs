impl TextAreaState {
    pub fn undo(&mut self) -> bool {
        let Some(snapshot) = self.advanced.undo.pop_back() else {
            return false;
        };
        self.push_redo(self.snapshot());
        self.restore(snapshot);
        self.after_history_change();
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(snapshot) = self.advanced.redo.pop_back() else {
            return false;
        };
        self.push_undo(self.snapshot());
        self.restore(snapshot);
        self.after_history_change();
        true
    }

    pub fn copy_selection_or_line(&mut self) -> String {
        let value = self.selected_text().map(str::to_owned).unwrap_or_else(|| {
            self.text[line_start(&self.text, self.cursor)..line_end(&self.text, self.cursor)]
                .to_owned()
        });
        self.advanced.clipboard.clone_from(&value);
        value
    }

    pub fn search(
        &mut self,
        query: impl Into<String>,
        case_sensitive: bool,
    ) -> Result<(), TextAreaError> {
        let query = query.into();
        if query.is_empty() {
            self.advanced.search = TextAreaSearchState::default();
            return Err(TextAreaError::EmptySearch);
        }
        self.advanced.search.query = bounded_utf8(query, 4_096);
        self.advanced.search.case_sensitive = case_sensitive;
        self.rebuild_search();
        if let Some(index) = self
            .advanced
            .search
            .matches
            .iter()
            .position(|(start, _)| *start >= self.cursor)
        {
            self.select_match(index);
        } else if !self.advanced.search.matches.is_empty() {
            self.select_match(0);
        }
        Ok(())
    }

    pub fn next_match(&mut self, backwards: bool) -> bool {
        if self.advanced.search.matches.is_empty() {
            return false;
        }
        let current = self.advanced.search.selected.unwrap_or(0);
        let next = if backwards {
            current
                .checked_sub(1)
                .unwrap_or(self.advanced.search.matches.len() - 1)
        } else {
            (current + 1) % self.advanced.search.matches.len()
        };
        self.select_match(next);
        true
    }

    pub fn replace_selected_match(&mut self, replacement: &str) -> Result<bool, TextAreaError> {
        let Some(index) = self.advanced.search.selected else {
            return Ok(false);
        };
        let Some(&(start, end)) = self.advanced.search.matches.get(index) else {
            return Ok(false);
        };
        self.select_range(start, end);
        self.try_insert(replacement)?;
        self.rebuild_search();
        if !self.advanced.search.matches.is_empty() {
            let next = index.min(self.advanced.search.matches.len() - 1);
            self.select_match(next);
        }
        Ok(true)
    }

    pub fn replace_all(&mut self, replacement: &str) -> Result<usize, TextAreaError> {
        let matches = self.advanced.search.matches.clone();
        if matches.is_empty() {
            return Ok(0);
        }
        let removed: usize = matches.iter().map(|(start, end)| end - start).sum();
        let final_len = self
            .text
            .len()
            .saturating_sub(removed)
            .saturating_add(replacement.len().saturating_mul(matches.len()));
        if final_len > TEXTAREA_MAX_BYTES {
            return Err(TextAreaError::TextLimit {
                limit: TEXTAREA_MAX_BYTES,
            });
        }
        self.remember();
        for (start, end) in matches.iter().rev() {
            self.text.replace_range(*start..*end, replacement);
        }
        self.cursor = self.cursor.min(self.text.len());
        self.selection = None;
        self.after_edit();
        self.rebuild_search();
        Ok(matches.len())
    }

}
