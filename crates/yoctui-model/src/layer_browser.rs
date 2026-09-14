//! Layer browser.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerRelationships {
    pub layers: Vec<LayerRelationship>,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerRelationship {
    pub name: String,
    pub priority: Option<i32>,
    pub compatible: Vec<String>,
    pub depends: Vec<String>,
    pub overlays: Vec<String>,
    pub appends: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GitFileState {
    Clean,
    Modified,
    Untracked,
    Ignored,
    #[default]
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PreviewKind {
    Text,
    Binary,
    #[default]
    Unavailable,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LayerInspectorMode {
    #[default]
    Preview,
    Git,
    Metadata,
    Dependencies,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayerBrowserEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
    pub is_hidden: bool,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    pub git: GitFileState,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerBrowser {
    pub layer: String,
    pub root: PathBuf,
    pub directory: PathBuf,
    pub entries: Vec<LayerBrowserEntry>,
    pub nodes: HashMap<PathBuf, Vec<LayerBrowserEntry>>,
    pub expanded: HashSet<PathBuf>,
    pub show_hidden: bool,
    pub selection: usize,
    pub preview: String,
    pub preview_kind: PreviewKind,
    pub preview_truncated: bool,
    pub preview_scroll: usize,
    pub preview_focused: bool,
    pub inspector_mode: LayerInspectorMode,
    pub tree_truncated: bool,
    pub cycle_entries: usize,
}
impl LayerBrowser {
    pub fn new(layer: String, root: PathBuf) -> Self {
        let mut expanded = HashSet::new();
        expanded.insert(root.clone());
        Self {
            layer,
            directory: root.clone(),
            root,
            entries: Vec::new(),
            nodes: HashMap::new(),
            expanded,
            show_hidden: false,
            selection: 0,
            preview: String::new(),
            preview_kind: PreviewKind::Unavailable,
            preview_truncated: false,
            preview_scroll: 0,
            preview_focused: false,
            inspector_mode: LayerInspectorMode::Preview,
            tree_truncated: false,
            cycle_entries: 0,
        }
    }
    pub fn selected_entry(&self) -> Option<&LayerBrowserEntry> {
        self.entries.get(self.selection)
    }
    pub(crate) fn rebuild(&mut self, preferred: Option<&PathBuf>) {
        fn collect(
            directory: &PathBuf,
            depth: usize,
            nodes: &HashMap<PathBuf, Vec<LayerBrowserEntry>>,
            expanded: &HashSet<PathBuf>,
            show_hidden: bool,
            state: &mut (Vec<LayerBrowserEntry>, HashSet<PathBuf>, bool, usize),
        ) {
            if depth > LIST_TREE_MAX_DEPTH || state.0.len() >= LIST_TREE_MAX_ROWS {
                state.2 = true;
                return;
            }
            if !state.1.insert(directory.clone()) {
                state.3 += 1;
                return;
            }
            let Some(children) = nodes.get(directory) else {
                state.1.remove(directory);
                return;
            };
            for child in children {
                if state.0.len() == LIST_TREE_MAX_ROWS {
                    state.2 = true;
                    break;
                }
                if child.is_hidden && !show_hidden {
                    continue;
                }
                let mut visible = child.clone();
                visible.depth = depth;
                state.0.push(visible);
                if child.is_dir && expanded.contains(&child.path) {
                    collect(&child.path, depth + 1, nodes, expanded, show_hidden, state);
                }
            }
            state.1.remove(directory);
        }
        let mut state = (Vec::new(), HashSet::new(), false, 0usize);
        collect(
            &self.root,
            0,
            &self.nodes,
            &self.expanded,
            self.show_hidden,
            &mut state,
        );
        let (entries, _, truncated, cycles) = state;
        self.entries = entries;
        self.tree_truncated = truncated;
        self.cycle_entries = cycles;
        self.selection = preferred
            .and_then(|path| self.entries.iter().position(|entry| &entry.path == path))
            .unwrap_or_else(|| self.selection.min(self.entries.len().saturating_sub(1)));
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePicker {
    pub images: Vec<String>,
    pub selection: usize,
}
pub const MAX_BUILD_HISTORY: usize = 50;
impl Default for BuildState {
    fn default() -> Self {
        Self {
            status: BuildStatus::Idle,
            target: None,
            started: None,
            completed: 0,
            total: None,
            parse_current: None,
            parse_total: None,
            warnings: 0,
            errors: 0,
            exit_code: None,
        }
    }
}
