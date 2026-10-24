//! Persistent, client-local development workspaces. Processes remain daemon-owned.
use crate::*;
mod debug;
mod persistence;
mod reducer;
mod viewport;
pub use debug::*;
pub use persistence::*;
pub use viewport::*;

pub const WORKBENCH_MAX_SESSIONS: usize = 16;
pub const WORKBENCH_MAX_FOLDERS: usize = 8;
pub const WORKBENCH_MAX_TABS: usize = 16;
pub const WORKBENCH_MAX_TEXT: usize = 8 * 1024 * 1024;
pub const WORKBENCH_TAB_WIDTH: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkbenchFocus {
    #[default]
    Files,
    Editor,
    Output,
    Terminal,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkbenchOutput {
    #[default]
    Build,
    Terminal,
    Changes,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkbenchFormKind {
    Create,
    Rename,
    AddFolder,
    NewFile,
    Recipe,
    Find,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchForm {
    pub kind: WorkbenchFormKind,
    pub text: String,
}
/// Transient directory chooser; filesystem reads run in the CLI worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchFolderBrowser {
    pub directory: PathBuf,
    pub entries: Vec<PathBuf>,
    /// Zero selects the current directory; child directories follow it.
    pub selection: usize,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
    pub editing_path: bool,
}
impl WorkbenchFolderBrowser {
    pub fn selected_path(&self) -> &std::path::Path {
        self.selection
            .checked_sub(1)
            .and_then(|index| self.entries.get(index))
            .unwrap_or(&self.directory)
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchTab {
    pub root: PathBuf,
    pub path: PathBuf,
    pub document: TextAreaState,
    pub original: String,
    pub error: Option<String>,
    pub diff: Option<TextAreaDiffPreview>,
    /// High-water reservation covers undo/redo without evicting unfinished edits.
    pub reserved: usize,
    pub saving: bool,
    pub viewport: WorkbenchViewport,
    pub breakpoints: BTreeSet<usize>,
}
impl WorkbenchTab {
    pub fn is_dirty(&self) -> bool {
        self.document.text != self.original
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchFolder {
    pub browser: LayerBrowser,
    pub request: u64,
    pub requests: HashMap<PathBuf, u64>,
    pub loading: bool,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkbenchSession {
    pub id: u64,
    pub name: String,
    pub build_dir: Option<PathBuf>,
    pub recipe: Option<String>,
    pub folders: Vec<WorkbenchFolder>,
    pub folder: usize,
    pub tabs: Vec<WorkbenchTab>,
    pub tab: usize,
    pub focus: WorkbenchFocus,
    pub output: WorkbenchOutput,
    pub panel_zoom: bool,
    pub terminal: Option<u64>,
    pub terminal_daemon: Option<DaemonModelInstanceId>,
    pub show_whitespace: bool,
    pub debug: WorkbenchDebugConfig,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkbenchState {
    pub sessions: Vec<WorkbenchSession>,
    pub selection: usize,
    pub active: Option<u64>,
    pub generation: u64,
    pub file_request: u64,
    pub pending_folders: HashMap<u64, u64>,
    pub form: Option<WorkbenchForm>,
    pub folder_browser: Option<WorkbenchFolderBrowser>,
    pub menu: Option<usize>,
    pub revision: u64,
    pub persisted_revision: u64,
    pub persistence_error: Option<String>,
    pub recovering: bool,
    pub deferred_recipe: Option<(String, PathBuf)>,
    pub tab_picker: Option<usize>,
    pub debug_dialog: Option<WorkbenchDebugDialog>,
    pub debug_request: u64,
    pub pending_compile: Option<(u64, String, Option<PathBuf>)>,
}
impl WorkbenchState {
    pub fn modal_open(&self) -> bool {
        self.form.is_some()
            || self.menu.is_some()
            || self.debug_dialog.is_some()
            || self.tab_picker.is_some()
    }
    pub fn current(&self) -> Option<&WorkbenchSession> {
        self.sessions.iter().find(|s| Some(s.id) == self.active)
    }
    pub fn current_mut(&mut self) -> Option<&mut WorkbenchSession> {
        self.sessions.iter_mut().find(|s| Some(s.id) == self.active)
    }
    pub fn changed(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }
    pub fn text_bytes(&self) -> usize {
        self.sessions
            .iter()
            .flat_map(|s| &s.tabs)
            .map(|t| t.reserved.max(t.original.len()).saturating_mul(2))
            .sum()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbenchRequest {
    PrepareDebug {
        session: u64,
        generation: u64,
        root: PathBuf,
        config: WorkbenchDebugConfig,
        breakpoints: Vec<(PathBuf, usize)>,
    },
    BrowseFolders {
        path: PathBuf,
        generation: u64,
    },
    Folder {
        session: u64,
        generation: u64,
        path: PathBuf,
    },
    Directory {
        session: u64,
        root: PathBuf,
        directory: PathBuf,
        generation: u64,
    },
    Read {
        session: u64,
        root: PathBuf,
        path: PathBuf,
        generation: u64,
    },
    Save {
        session: u64,
        root: PathBuf,
        path: PathBuf,
        content: String,
        expected: TextAreaRevision,
    },
    NewFile {
        session: u64,
        root: PathBuf,
        path: PathBuf,
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkbenchAction {
    Compile,
    Utility(DevtoolUtilityCommand),
    ToggleWhitespace,
    SearchMatch {
        backwards: bool,
    },
    Tabs,
    TabPickerSelect(isize),
    TabPickerOpen,
    TabPickerClose,
    ScrollEditor(isize),
    Viewport {
        rows: usize,
        columns: usize,
    },
    ToggleBreakpoint,
    BreakpointAt(usize),
    Debug,
    DebugField(isize),
    DebugText(char),
    DebugBackspace,
    DebugClear,
    DebugMode,
    DebugReview,
    DebugPrepared {
        session: u64,
        generation: u64,
        result: Result<TerminalLaunchRequest, String>,
    },
    BrowseFolders,
    BrowseSelect(isize),
    BrowseSelectAt(usize),
    BrowseOpen,
    BrowseParent,
    BrowseChoose,
    FoldersBrowsed {
        generation: u64,
        result: Result<(PathBuf, Vec<PathBuf>), String>,
    },
    Form(WorkbenchFormKind),
    FormText(char),
    FormBackspace,
    FormClear,
    ConfirmForm,
    Cancel,
    Select(isize),
    SelectAt(usize),
    Open,
    Sessions,
    RemoveSession,
    RemoveFolder,
    Focus(WorkbenchFocus),
    CycleFocus,
    Folder(isize),
    FolderAt(usize),
    Expand,
    Collapse,
    Hidden,
    Refresh,
    Tab(isize),
    SelectTab(usize),
    CloseTab,
    Save,
    Edit(PopupEditorCommand),
    SelectAll,
    Cut,
    Menu,
    MenuSelect(isize),
    MenuActivate,
    Changes,
    TogglePanel,
    Terminal,
    ReleaseTerminal,
    RecipeAction(Box<Action>),
    OpenRecipe {
        recipe: String,
        root: PathBuf,
    },
    FolderLoaded {
        session: u64,
        generation: u64,
        result: Result<PathBuf, String>,
    },
    DirectoryLoaded {
        session: u64,
        root: PathBuf,
        directory: PathBuf,
        generation: u64,
        result: Result<Vec<LayerBrowserEntry>, String>,
    },
    FileLoaded {
        session: u64,
        root: PathBuf,
        path: PathBuf,
        generation: u64,
        result: Result<String, String>,
    },
    Saved {
        session: u64,
        path: PathBuf,
        content: String,
        result: Result<(), String>,
    },
    NewFileCreated {
        session: u64,
        root: PathBuf,
        path: PathBuf,
        result: Result<(), String>,
    },
}

pub const WORKBENCH_MENU: &[&str] = &[
    "Add folder…",
    "New file…",
    "Bind recipe…",
    "Build recipe…",
    "Devtool modify…",
    "Refresh Devtool status",
    "Devshell…",
    "Workspace terminal…",
    "Review changes",
    "Create/update patches…",
    "Finish into layer…",
    "Reset recipe…",
    "Rename workspace…",
    "Remove selected folder",
    "Compile recipe (BitBake do_compile)… · Alt+c",
    "Build modified recipe (devtool build)…",
    "Prepare recipe toolchain (devtool ide-sdk)… · Alt+i",
    "Configure / launch GDB… · Alt+g / F5",
    "Toggle breakpoint on current line · Alt+k / F9",
    "Show/hide whitespace · Alt+w",
    "Switch editor tab… · Alt+v",
    "Find in current file… · Ctrl+f",
    "Next search match · Ctrl+g / Alt+n",
    "Previous search match · Alt+p",
    "Back to sessions",
];

pub fn workbench_reduce(app: &mut App, action: WorkbenchAction) -> Option<Effect> {
    reducer::reduce(app, action)
}
