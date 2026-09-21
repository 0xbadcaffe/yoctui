impl TextAreaState {
    pub fn accept_external_text(&mut self, text: String) {
        *self = Self::new(text);
    }

    /// Accept content already saved by an external editor. The disk baseline
    /// advances so build actions are not blocked, while the prior visual
    /// baseline is retained so the in-TUI diff remains meaningful.
    pub fn accept_external_edit(&mut self, mut text: String) {
        truncate_utf8(&mut text, TEXTAREA_MAX_BYTES);
        self.text = text;
        self.advanced.base_text.clone_from(&self.text);
        self.advanced.save = TextAreaSaveState::Clean {
            revision: TextAreaRevision::of(&self.text),
        };
        self.cursor = self.text.len();
        self.selection = None;
        self.editing = false;
        self.advanced.mode = TextAreaMode::Normal;
        self.after_history_change();
    }

    fn snapshot(&self) -> TextAreaSnapshot {
        TextAreaSnapshot {
            text: self.text.clone(),
            cursor: self.cursor,
            selection: self.selection,
        }
    }

    fn restore(&mut self, snapshot: TextAreaSnapshot) {
        self.text = snapshot.text;
        self.cursor = clamp_boundary(&self.text, snapshot.cursor);
        self.selection = snapshot.selection.map(|(start, end)| {
            (
                clamp_boundary(&self.text, start),
                clamp_boundary(&self.text, end),
            )
        });
        self.advanced.visual_anchor = self
            .advanced
            .visual_anchor
            .map(|anchor| clamp_boundary(&self.text, anchor));
    }

    fn remember(&mut self) {
        let snapshot = self.snapshot();
        if self.advanced.undo.back() != Some(&snapshot) {
            self.push_undo(snapshot);
        }
        self.advanced.redo.clear();
    }

    fn push_undo(&mut self, snapshot: TextAreaSnapshot) {
        push_bounded(&mut self.advanced.undo, snapshot);
    }

    fn push_redo(&mut self, snapshot: TextAreaSnapshot) {
        push_bounded(&mut self.advanced.redo, snapshot);
    }

    fn after_edit(&mut self) {
        self.cursor = clamp_boundary(&self.text, self.cursor);
        self.advanced.preferred_column = None;
        self.advanced.visual_anchor = None;
        self.selection = None;
        if self.advanced.mode == TextAreaMode::Visual {
            self.advanced.mode = TextAreaMode::Normal;
        }
        self.advanced.validation.clear();
        self.rebuild_search();
        self.after_history_change();
    }

    fn after_history_change(&mut self) {
        self.rebuild_search();
        let base = TextAreaRevision::of(&self.advanced.base_text);
        let current = TextAreaRevision::of(&self.text);
        self.advanced.save = if base == current {
            TextAreaSaveState::Clean { revision: current }
        } else {
            TextAreaSaveState::Modified { base, current }
        };
    }

    fn rebuild_search(&mut self) {
        self.advanced.search.matches.clear();
        self.advanced.search.selected = None;
        self.advanced.search.truncated = false;
        if self.advanced.search.query.is_empty() {
            return;
        }
        let matches = if self.advanced.search.case_sensitive {
            self.text
                .match_indices(&self.advanced.search.query)
                .map(|(start, matched)| (start, start + matched.len()))
                .collect::<Vec<_>>()
        } else {
            unicode_case_insensitive_matches(&self.text, &self.advanced.search.query)
        };
        self.advanced.search.truncated = matches.len() > TEXTAREA_MAX_SEARCH_MATCHES;
        self.advanced
            .search
            .matches
            .extend(matches.into_iter().take(TEXTAREA_MAX_SEARCH_MATCHES));
    }

    fn select_match(&mut self, index: usize) {
        if let Some(&(start, end)) = self.advanced.search.matches.get(index) {
            self.advanced.search.selected = Some(index);
            self.selection = Some((start, end));
            self.cursor = end;
        }
    }
}
