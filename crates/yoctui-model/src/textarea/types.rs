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
    diff_base_text: String,
}

