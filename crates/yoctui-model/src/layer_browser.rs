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
    pub rootfs_metadata: Option<RootfsFileMetadata>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RootfsFileMetadata {
    pub kind: RootfsEntryKind,
    pub mode: Option<u32>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub link_target: Option<PathBuf>,
}
impl RootfsFileMetadata {
    pub fn permissions(&self) -> String {
        let Some(mode) = self.mode else {
            return "??????????".into();
        };
        let mut result = String::from(match self.kind {
            RootfsEntryKind::Directory => "d",
            RootfsEntryKind::RegularFile => "-",
            RootfsEntryKind::Symlink => "l",
            RootfsEntryKind::Other => match mode & 0o170000 {
                0o010000 => "p",
                0o020000 => "c",
                0o060000 => "b",
                0o140000 => "s",
                _ => "?",
            },
        });
        for shift in [6, 3, 0] {
            result.push(if mode & (4 << shift) != 0 { 'r' } else { '-' });
            result.push(if mode & (2 << shift) != 0 { 'w' } else { '-' });
            let executable = mode & (1 << shift) != 0;
            let special =
                mode & match shift {
                    6 => 0o4000,
                    3 => 0o2000,
                    _ => 0o1000,
                } != 0;
            result.push(match (special, executable, shift) {
                (true, true, 0) => 't',
                (true, false, 0) => 'T',
                (true, true, _) => 's',
                (true, false, _) => 'S',
                (false, true, _) => 'x',
                _ => '-',
            });
        }
        result
    }
    pub fn listing(&self, size: Option<u64>) -> String {
        format!(
            "{} {} {} {} {} B",
            self.permissions(),
            self.mode.map_or_else(
                || "mode unavailable".into(),
                |mode| format!("{:04o}", mode & 0o7777)
            ),
            account_label(self.owner.as_deref(), self.uid),
            account_label(self.group.as_deref(), self.gid),
            size.map_or_else(|| "?".into(), |value| value.to_string())
        )
    }
}
fn account_label(name: Option<&str>, id: Option<u32>) -> String {
    id.map_or_else(
        || "unavailable".into(),
        |id| name.map_or_else(|| id.to_string(), |name| format!("{name}({id})")),
    )
}
impl LayerBrowserEntry {
    pub fn can_preview(&self) -> bool {
        !self.is_dir
            && self
                .rootfs_metadata
                .as_ref()
                .is_none_or(|metadata| metadata.kind == RootfsEntryKind::RegularFile)
    }
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
    pub fn is_rootfs(&self) -> bool {
        self.layer.starts_with("Rootfs:") || self.layer == "Rootfs system"
    }
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
impl App {
    pub fn rootfs_browser_active(&self) -> bool {
        self.screen == Screen::Images
            && self.images_view == ImagesView::RootfsFilesystem
            && self.rootfs_browser().is_some()
    }
    pub fn rootfs_browser(&self) -> Option<&LayerBrowser> {
        self.layer_browser.as_ref().filter(|browser| {
            browser.is_rootfs()
                && self
                    .rootfs_composition
                    .composition()
                    .and_then(|composition| composition.root_directory.as_ref())
                    == Some(&browser.root)
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImagePicker {
    pub images: Vec<String>,
    pub selection: usize,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecipePickerPurpose {
    Build,
    Dependencies,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecipePicker {
    pub recipes: Vec<RecipeIdentity>,
    pub selection: usize,
    pub purpose: RecipePickerPurpose,
}
pub const MAX_BUILD_HISTORY: usize = 50;
impl Default for BuildState {
    fn default() -> Self {
        Self {
            cache: BuildCacheState::default(),
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
