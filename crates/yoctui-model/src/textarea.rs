use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

pub const TEXTAREA_MAX_BYTES: usize = 1_048_576;
pub const TEXTAREA_MAX_HISTORY: usize = 64;
pub const TEXTAREA_MAX_SEARCH_MATCHES: usize = 2_048;
pub const TEXTAREA_MAX_VALIDATION_SPANS: usize = 512;
pub const TEXTAREA_MAX_DIFF_LINES: usize = 4_096;
pub const TEXTAREA_MAX_PASTE_BYTES: usize = 262_144;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAreaMode {
    #[default]
    Normal,
    Insert,
    Visual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaPasteSource {
    Clipboard,
    BracketedPaste,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaMotion {
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    LineStart,
    LineEnd,
    PageUp,
    PageDown,
    DocumentStart,
    DocumentEnd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAreaPosition {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextAreaError {
    TextLimit { limit: usize },
    PasteLimit { limit: usize },
    EmptySearch,
    NoSaveFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaValidationSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaValidationSpan {
    pub start: usize,
    pub end: usize,
    pub severity: TextAreaValidationSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TextAreaSearchState {
    pub query: String,
    pub case_sensitive: bool,
    pub matches: Vec<(usize, usize)>,
    pub selected: Option<usize>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAreaVisualLine {
    pub source_line: usize,
    pub start: usize,
    pub end: usize,
    pub continuation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaLayout {
    pub line_numbers: bool,
    pub wrap_width: Option<usize>,
    pub viewport_rows: usize,
}

impl Default for TextAreaLayout {
    fn default() -> Self {
        Self {
            line_numbers: true,
            wrap_width: None,
            viewport_rows: 20,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextAreaRevision {
    pub bytes: usize,
    pub sha256: [u8; 32],
}

impl TextAreaRevision {
    pub fn of(text: &str) -> Self {
        Self {
            bytes: text.len(),
            sha256: Sha256::digest(text.as_bytes()).into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAreaDiffKind {
    Context,
    Removed,
    Added,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaDiffLine {
    pub kind: TextAreaDiffKind,
    pub old_line: Option<usize>,
    pub new_line: Option<usize>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaDiffPreview {
    pub base: TextAreaRevision,
    pub current: TextAreaRevision,
    pub lines: Vec<TextAreaDiffLine>,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaAtomicSave {
    pub target: PathBuf,
    pub temporary: PathBuf,
    pub content: String,
    pub revision: TextAreaRevision,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextAreaSaveState {
    Clean {
        revision: TextAreaRevision,
    },
    Modified {
        base: TextAreaRevision,
        current: TextAreaRevision,
    },
    Preview {
        preview: TextAreaDiffPreview,
    },
    Conflict {
        target: PathBuf,
        expected: TextAreaRevision,
        observed: TextAreaRevision,
    },
    Saving {
        target: PathBuf,
        temporary: PathBuf,
        revision: TextAreaRevision,
    },
    Saved {
        target: PathBuf,
        revision: TextAreaRevision,
    },
    Failed {
        target: PathBuf,
        temporary: PathBuf,
        revision: TextAreaRevision,
        message: String,
        recoverable: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextAreaSnapshot {
    text: String,
    cursor: usize,
    selection: Option<(usize, usize)>,
}

/// Reducer-owned editor state. Byte offsets are always valid UTF-8 boundaries.
///
/// `editing` is retained as a compatibility projection for existing dialogs;
/// new code should use [`Self::mode`] and [`Self::set_mode`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextAreaState {
    pub text: String,
    pub cursor: usize,
    pub selection: Option<(usize, usize)>,
    pub editing: bool,
    advanced: Box<TextAreaAdvancedState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TextAreaAdvancedState {
    layout: TextAreaLayout,
    search: TextAreaSearchState,
    validation: Vec<TextAreaValidationSpan>,
    save: TextAreaSaveState,
    mode: TextAreaMode,
    visual_anchor: Option<usize>,
    preferred_column: Option<usize>,
    clipboard: String,
    undo: VecDeque<TextAreaSnapshot>,
    redo: VecDeque<TextAreaSnapshot>,
    base_text: String,
}

impl TextAreaState {
    pub fn new(mut text: String) -> Self {
        truncate_utf8(&mut text, TEXTAREA_MAX_BYTES);
        let cursor = text.len();
        let revision = TextAreaRevision::of(&text);
        let base_text = text.clone();
        Self {
            text,
            cursor,
            selection: None,
            editing: false,
            advanced: Box::new(TextAreaAdvancedState {
                base_text,
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
        self.base_revision() != TextAreaRevision::of(&self.text)
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

    pub fn set_validation<I>(&mut self, spans: I)
    where
        I: IntoIterator<Item = TextAreaValidationSpan>,
    {
        self.advanced.validation = spans
            .into_iter()
            .take(TEXTAREA_MAX_VALIDATION_SPANS)
            .map(|mut span| {
                span.start = clamp_boundary(&self.text, span.start);
                span.end = clamp_boundary(&self.text, span.end);
                if span.start > span.end {
                    std::mem::swap(&mut span.start, &mut span.end);
                }
                span.message = bounded_utf8(span.message, 4_096);
                span
            })
            .collect();
    }

    pub fn visual_lines(&self, first: usize, limit: usize) -> Vec<TextAreaVisualLine> {
        let cap = limit.min(4_096);
        if cap == 0 {
            return Vec::new();
        }
        let mut projected = Vec::with_capacity(cap);
        let mut visual_index = 0usize;
        for (source_line, (start, end)) in line_ranges(&self.text).into_iter().enumerate() {
            let width = self.advanced.layout.wrap_width.filter(|width| *width > 0);
            let chunks = wrap_ranges(&self.text, start, end, width);
            for (index, (chunk_start, chunk_end)) in chunks.into_iter().enumerate() {
                if visual_index >= first {
                    projected.push(TextAreaVisualLine {
                        source_line,
                        start: chunk_start,
                        end: chunk_end,
                        continuation: index > 0,
                    });
                    if projected.len() == cap {
                        return projected;
                    }
                }
                visual_index = visual_index.saturating_add(1);
            }
        }
        projected
    }

    pub fn preview_diff(&mut self) -> &TextAreaDiffPreview {
        let preview = build_diff(&self.advanced.base_text, &self.text);
        self.advanced.save = TextAreaSaveState::Preview { preview };
        let TextAreaSaveState::Preview { preview } = &self.advanced.save else {
            unreachable!()
        };
        preview
    }

    pub fn begin_atomic_save(
        &mut self,
        target: impl Into<PathBuf>,
        observed: TextAreaRevision,
    ) -> Option<TextAreaAtomicSave> {
        let target = target.into();
        let expected = TextAreaRevision::of(&self.advanced.base_text);
        if observed != expected {
            self.advanced.save = TextAreaSaveState::Conflict {
                target,
                expected,
                observed,
            };
            return None;
        }
        let revision = TextAreaRevision::of(&self.text);
        let temporary = atomic_temporary_path(&target, &revision);
        self.advanced.save = TextAreaSaveState::Saving {
            target: target.clone(),
            temporary: temporary.clone(),
            revision,
        };
        Some(TextAreaAtomicSave {
            target,
            temporary,
            content: self.text.clone(),
            revision,
        })
    }

    pub fn mark_saved(&mut self, request: &TextAreaAtomicSave) -> bool {
        let matches = matches!(
            &self.advanced.save,
            TextAreaSaveState::Saving { target, temporary, revision }
                if target == &request.target && temporary == &request.temporary && revision == &request.revision
        ) && TextAreaRevision::of(&self.text) == request.revision;
        if matches {
            self.advanced.base_text.clone_from(&self.text);
            self.advanced.save = TextAreaSaveState::Saved {
                target: request.target.clone(),
                revision: request.revision,
            };
        }
        matches
    }

    pub fn mark_save_failed(&mut self, message: impl Into<String>, recoverable: bool) -> bool {
        let TextAreaSaveState::Saving {
            target,
            temporary,
            revision,
        } = &self.advanced.save
        else {
            return false;
        };
        self.advanced.save = TextAreaSaveState::Failed {
            target: target.clone(),
            temporary: temporary.clone(),
            revision: *revision,
            message: bounded_utf8(message.into(), 8_192),
            recoverable,
        };
        true
    }

    pub fn retry_save(&mut self) -> Result<TextAreaAtomicSave, TextAreaError> {
        let TextAreaSaveState::Failed {
            target,
            temporary,
            revision,
            recoverable,
            ..
        } = &self.advanced.save
        else {
            return Err(TextAreaError::NoSaveFailure);
        };
        if !recoverable {
            return Err(TextAreaError::NoSaveFailure);
        }
        let request = TextAreaAtomicSave {
            target: target.clone(),
            temporary: temporary.clone(),
            content: self.text.clone(),
            revision: *revision,
        };
        self.advanced.save = TextAreaSaveState::Saving {
            target: request.target.clone(),
            temporary: request.temporary.clone(),
            revision: request.revision,
        };
        Ok(request)
    }

    pub fn accept_external_text(&mut self, text: String) {
        *self = Self::new(text);
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

fn push_bounded(queue: &mut VecDeque<TextAreaSnapshot>, snapshot: TextAreaSnapshot) {
    if queue.len() == TEXTAREA_MAX_HISTORY {
        queue.pop_front();
    }
    queue.push_back(snapshot);
}

fn position_at(text: &str, offset: usize) -> TextAreaPosition {
    let offset = clamp_boundary(text, offset);
    let start = line_start(text, offset);
    TextAreaPosition {
        line: text[..start].bytes().filter(|byte| *byte == b'\n').count(),
        column: text[start..offset].chars().count(),
    }
}

fn offset_for_position(text: &str, target_line: usize, column: usize) -> usize {
    let mut start = 0;
    for _ in 0..target_line {
        start = text[start..]
            .find('\n')
            .map_or(text.len(), |index| start + index + 1);
    }
    let end = line_end(text, start);
    text[start..end]
        .char_indices()
        .nth(column)
        .map_or(end, |(index, _)| start + index)
}

fn line_start(text: &str, cursor: usize) -> usize {
    text[..cursor].rfind('\n').map_or(0, |index| index + 1)
}

fn line_end(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .find('\n')
        .map_or(text.len(), |index| cursor + index)
}

fn previous_boundary(text: &str, cursor: usize) -> usize {
    text[..cursor]
        .char_indices()
        .next_back()
        .map_or(0, |(index, _)| index)
}

fn next_boundary(text: &str, cursor: usize) -> usize {
    text[cursor..]
        .chars()
        .next()
        .map_or(text.len(), |character| cursor + character.len_utf8())
}

fn clamp_boundary(text: &str, mut index: usize) -> usize {
    index = index.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn word_left(text: &str, cursor: usize) -> usize {
    let mut at = cursor;
    while at > 0 {
        let previous = previous_boundary(text, at);
        if !text[previous..at]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            break;
        }
        at = previous;
    }
    let class = (at > 0)
        .then(|| {
            let previous = previous_boundary(text, at);
            text[previous..at].chars().next().map(word_class)
        })
        .flatten();
    while at > 0 {
        let previous = previous_boundary(text, at);
        let Some(character) = text[previous..at].chars().next() else {
            break;
        };
        if Some(word_class(character)) != class {
            break;
        }
        at = previous;
    }
    at
}

fn unicode_case_insensitive_matches(text: &str, query: &str) -> Vec<(usize, usize)> {
    let folded_query = query.to_lowercase();
    let mut matches = Vec::new();
    for (start, _) in text.char_indices() {
        let mut folded = String::new();
        for (relative, character) in text[start..].char_indices() {
            folded.extend(character.to_lowercase());
            if folded == folded_query {
                matches.push((start, start + relative + character.len_utf8()));
                break;
            }
            if folded.len() >= folded_query.len() && !folded_query.starts_with(&folded) {
                break;
            }
            if !folded_query.starts_with(&folded) {
                break;
            }
        }
    }
    matches
}

fn word_right(text: &str, cursor: usize) -> usize {
    let mut at = cursor;
    let class = text[at..].chars().next().map(word_class);
    while at < text.len() {
        let next = next_boundary(text, at);
        let Some(character) = text[at..next].chars().next() else {
            break;
        };
        if Some(word_class(character)) != class {
            break;
        }
        at = next;
    }
    while at < text.len() {
        let next = next_boundary(text, at);
        if !text[at..next]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            break;
        }
        at = next;
    }
    at
}

fn word_class(character: char) -> u8 {
    if character.is_alphanumeric() || character == '_' {
        1
    } else if character.is_whitespace() {
        2
    } else {
        3
    }
}

fn line_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for (index, character) in text.char_indices() {
        if character == '\n' {
            ranges.push((start, index));
            start = index + 1;
        }
    }
    ranges.push((start, text.len()));
    ranges
}

fn wrap_ranges(text: &str, start: usize, end: usize, width: Option<usize>) -> Vec<(usize, usize)> {
    let Some(width) = width else {
        return vec![(start, end)];
    };
    if start == end {
        return vec![(start, end)];
    }
    let mut ranges = Vec::new();
    let mut chunk_start = start;
    let mut chars = 0;
    for (relative, _) in text[start..end].char_indices() {
        if chars == width {
            ranges.push((chunk_start, start + relative));
            chunk_start = start + relative;
            chars = 0;
        }
        chars += 1;
    }
    ranges.push((chunk_start, end));
    ranges
}

fn build_diff(base: &str, current: &str) -> TextAreaDiffPreview {
    let old: Vec<_> = base.lines().collect();
    let new: Vec<_> = current.lines().collect();
    let mut prefix = 0;
    while prefix < old.len() && prefix < new.len() && old[prefix] == new[prefix] {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < old.len().saturating_sub(prefix)
        && suffix < new.len().saturating_sub(prefix)
        && old[old.len() - 1 - suffix] == new[new.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let mut lines = Vec::new();
    let context_start = prefix.saturating_sub(2);
    for (index, text) in old.iter().enumerate().take(prefix).skip(context_start) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Context,
            old_line: Some(index + 1),
            new_line: Some(index + 1),
            text: (*text).to_owned(),
        });
    }
    for (index, text) in old.iter().enumerate().take(old.len() - suffix).skip(prefix) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Removed,
            old_line: Some(index + 1),
            new_line: None,
            text: (*text).to_owned(),
        });
    }
    for (index, text) in new.iter().enumerate().take(new.len() - suffix).skip(prefix) {
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Added,
            old_line: None,
            new_line: Some(index + 1),
            text: (*text).to_owned(),
        });
    }
    for offset in (0..suffix)
        .rev()
        .take(2)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        let old_index = old.len() - suffix + offset;
        let new_index = new.len() - suffix + offset;
        lines.push(TextAreaDiffLine {
            kind: TextAreaDiffKind::Context,
            old_line: Some(old_index + 1),
            new_line: Some(new_index + 1),
            text: old[old_index].to_owned(),
        });
    }
    let truncated = lines.len() > TEXTAREA_MAX_DIFF_LINES;
    lines.truncate(TEXTAREA_MAX_DIFF_LINES);
    TextAreaDiffPreview {
        base: TextAreaRevision::of(base),
        current: TextAreaRevision::of(current),
        lines,
        truncated,
    }
}

fn atomic_temporary_path(target: &Path, revision: &TextAreaRevision) -> PathBuf {
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("yoctui-save");
    let suffix = u64::from_be_bytes(revision.sha256[..8].try_into().expect("fixed digest"));
    target.with_file_name(format!(".{name}.yoctui-{suffix:016x}.tmp"))
}

fn truncate_utf8(value: &mut String, limit: usize) {
    if value.len() <= limit {
        return;
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
}

fn bounded_utf8(mut value: String, limit: usize) -> String {
    truncate_utf8(&mut value, limit);
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_textarea_unicode_motion_selection_and_bounded_history() {
        let mut editor = TextAreaState::new("αβ\n猫 dog".into());
        editor.move_cursor(TextAreaMotion::DocumentStart);
        editor.set_mode(TextAreaMode::Visual);
        editor.move_cursor(TextAreaMotion::Right);
        editor.move_cursor(TextAreaMotion::Right);
        assert_eq!(editor.selected_text(), Some("αβ"));
        editor.set_mode(TextAreaMode::Insert);
        editor.try_insert("🙂").unwrap();
        for _ in 0..(TEXTAREA_MAX_HISTORY + 20) {
            editor.try_insert("x").unwrap();
        }
        assert_eq!(editor.history_lengths().0, TEXTAREA_MAX_HISTORY);
        assert!(editor.text.is_char_boundary(editor.cursor));
    }

    #[test]
    fn ux_textarea_search_replace_undo_redo_remain_utf8_safe() {
        let mut editor = TextAreaState::new("café café\nCAFÉ".into());
        editor.search("café", true).unwrap();
        assert_eq!(editor.search_state().matches.len(), 2);
        assert!(editor.replace_selected_match("茶").unwrap());
        assert_eq!(editor.text, "茶 café\nCAFÉ");
        assert!(editor.undo());
        assert_eq!(editor.text, "café café\nCAFÉ");
        assert!(editor.redo());
        assert_eq!(editor.text, "茶 café\nCAFÉ");
        assert!(editor.text.is_char_boundary(editor.cursor));
    }

    #[test]
    fn ux_textarea_layout_projects_line_numbers_and_wrap_metadata() {
        let mut editor = TextAreaState::new("abcdef\n猫犬".into());
        editor.layout_mut().wrap_width = Some(3);
        editor.layout_mut().line_numbers = true;
        let lines = editor.visual_lines(0, 10);
        assert_eq!(lines.len(), 3);
        assert_eq!((lines[0].source_line, lines[0].continuation), (0, false));
        assert_eq!((lines[1].source_line, lines[1].continuation), (0, true));
        assert_eq!(&editor.text[lines[2].start..lines[2].end], "猫犬");
    }

    #[test]
    fn ux_textarea_validation_diff_conflict_and_recoverable_atomic_save_are_typed() {
        let mut editor = TextAreaState::new("A=1\nB=2\n".into());
        editor.set_validation([TextAreaValidationSpan {
            start: 2,
            end: 3,
            severity: TextAreaValidationSeverity::Error,
            message: "invalid value".into(),
        }]);
        assert_eq!(editor.validation().len(), 1);
        editor.select_range(2, 3);
        editor.try_insert("3").unwrap();
        let preview = editor.preview_diff().clone();
        assert!(
            preview
                .lines
                .iter()
                .any(|line| line.kind == TextAreaDiffKind::Removed)
        );
        assert!(
            preview
                .lines
                .iter()
                .any(|line| line.kind == TextAreaDiffKind::Added)
        );

        let base = TextAreaRevision::of("A=1\nB=2\n");
        let request = editor.begin_atomic_save("/tmp/config", base).unwrap();
        assert_eq!(request.temporary.parent(), request.target.parent());
        assert!(editor.mark_save_failed("disk full", true));
        let retry = editor.retry_save().unwrap();
        assert_eq!(retry, request);
        assert!(editor.mark_saved(&retry));

        editor.try_insert("x").unwrap();
        assert!(editor.begin_atomic_save("/tmp/config", base).is_none());
        assert!(matches!(
            editor.save_state(),
            TextAreaSaveState::Conflict { .. }
        ));
    }

    #[test]
    fn ux_textarea_page_word_and_line_motion_use_character_columns() {
        let mut editor = TextAreaState::new("one two\n猫犬\nlast line\nend".into());
        editor.layout_mut().viewport_rows = 2;
        editor.move_cursor(TextAreaMotion::DocumentStart);
        editor.move_cursor(TextAreaMotion::WordRight);
        assert_eq!(editor.position(), TextAreaPosition { line: 0, column: 4 });
        editor.move_cursor(TextAreaMotion::PageDown);
        assert_eq!(editor.position(), TextAreaPosition { line: 2, column: 4 });
        editor.move_cursor(TextAreaMotion::LineEnd);
        assert_eq!(editor.position().column, 9);
    }

    #[test]
    fn ux_textarea_rejects_oversized_paste_without_mutation() {
        let mut editor = TextAreaState::new("safe".into());
        let before = editor.clone();
        let error = editor.paste_text(
            &"x".repeat(TEXTAREA_MAX_PASTE_BYTES + 1),
            TextAreaPasteSource::BracketedPaste,
        );
        assert_eq!(
            error,
            Err(TextAreaError::PasteLimit {
                limit: TEXTAREA_MAX_PASTE_BYTES
            })
        );
        assert_eq!(editor, before);
    }

    #[cfg(feature = "proptest")]
    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn ux_textarea_adversarial_sequences_keep_utf8_boundaries(
                initial in ".{0,512}",
                operations in prop::collection::vec((0u8..12, ".{0,16}"), 0..400)
            ) {
                let mut editor = TextAreaState::new(initial);
                for (operation, value) in operations {
                    match operation {
                        0 => { let _ = editor.try_insert(&value); }
                        1 => editor.backspace(),
                        2 => editor.delete_forward(),
                        3 => editor.move_cursor(TextAreaMotion::Left),
                        4 => editor.move_cursor(TextAreaMotion::Right),
                        5 => editor.move_cursor(TextAreaMotion::Up),
                        6 => editor.move_cursor(TextAreaMotion::Down),
                        7 => { editor.undo(); }
                        8 => { editor.redo(); }
                        9 => { editor.set_mode(TextAreaMode::Visual); editor.move_cursor(TextAreaMotion::WordRight); }
                        10 => { let _ = editor.search(value, true); }
                        _ => { let _ = editor.replace_all(&value); }
                    }
                    prop_assert!(editor.text.len() <= TEXTAREA_MAX_BYTES);
                    prop_assert!(editor.text.is_char_boundary(editor.cursor));
                    if let Some((start, end)) = editor.selection {
                        prop_assert!(start <= end && end <= editor.text.len());
                        prop_assert!(editor.text.is_char_boundary(start));
                        prop_assert!(editor.text.is_char_boundary(end));
                    }
                    let (undo, redo) = editor.history_lengths();
                    prop_assert!(undo <= TEXTAREA_MAX_HISTORY);
                    prop_assert!(redo <= TEXTAREA_MAX_HISTORY);
                }
            }
        }
    }
}
