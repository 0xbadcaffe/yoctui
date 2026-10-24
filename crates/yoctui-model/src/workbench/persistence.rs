use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedWorkbenches {
    pub sessions: Vec<SavedWorkbenchSession>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedWorkbenchSession {
    pub id: u64,
    pub name: String,
    pub build_dir: Option<PathBuf>,
    pub recipe: Option<String>,
    pub folders: Vec<PathBuf>,
    pub folder: usize,
    pub tabs: Vec<SavedWorkbenchTab>,
    pub tab: usize,
    #[serde(default)]
    pub show_whitespace: bool,
    #[serde(default)]
    pub debug: WorkbenchDebugConfig,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SavedWorkbenchTab {
    pub root: PathBuf,
    pub path: PathBuf,
    pub original: String,
    pub text: String,
    pub cursor: usize,
    #[serde(default)]
    pub breakpoints: BTreeSet<usize>,
}
impl WorkbenchState {
    pub fn saved(&self) -> SavedWorkbenches {
        SavedWorkbenches {
            sessions: self
                .sessions
                .iter()
                .map(|s| SavedWorkbenchSession {
                    id: s.id,
                    name: s.name.clone(),
                    build_dir: s.build_dir.clone(),
                    recipe: s.recipe.clone(),
                    folders: s.folders.iter().map(|f| f.browser.root.clone()).collect(),
                    folder: s.folder,
                    tabs: s
                        .tabs
                        .iter()
                        .map(|t| SavedWorkbenchTab {
                            root: t.root.clone(),
                            path: t.path.clone(),
                            original: t.original.clone(),
                            text: t.document.text.clone(),
                            cursor: t.document.cursor,
                            breakpoints: t.breakpoints.clone(),
                        })
                        .collect(),
                    tab: s.tab,
                    show_whitespace: s.show_whitespace,
                    debug: s.debug.clone(),
                })
                .collect(),
        }
    }
    pub fn restore(saved: SavedWorkbenches) -> Result<Self, String> {
        saved.validate()?;
        let mut state = Self::default();
        for session in saved.sessions {
            let tabs = session
                .tabs
                .into_iter()
                .map(|t| {
                    let mut document = TextAreaState::new(t.original.clone());
                    if t.text != t.original {
                        document.select_range(0, document.text.len());
                        document
                            .try_insert(&t.text)
                            .map_err(|e| format!("Invalid recovery buffer: {e:?}"))?;
                    }
                    document.cursor = t.cursor.min(document.text.len());
                    while !document.text.is_char_boundary(document.cursor) {
                        document.cursor = document.cursor.saturating_sub(1);
                    }
                    document.set_mode(TextAreaMode::Insert);
                    Ok(WorkbenchTab {
                        reserved: t.text.len().max(t.original.len()),
                        saving: false,
                        viewport: WorkbenchViewport {
                            follow_cursor: true,
                            ..Default::default()
                        },
                        breakpoints: t.breakpoints,
                        root: t.root,
                        path: t.path,
                        original: t.original,
                        document,
                        error: None,
                        diff: None,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?;
            let folders = session
                .folders
                .into_iter()
                .map(|root| WorkbenchFolder {
                    browser: LayerBrowser::new("Workbench".into(), root),
                    request: 0,
                    requests: HashMap::new(),
                    loading: false,
                    error: None,
                })
                .collect();
            state.generation = state.generation.max(session.id);
            state.sessions.push(WorkbenchSession {
                id: session.id,
                name: session.name,
                build_dir: session.build_dir,
                recipe: session.recipe,
                folders,
                folder: session.folder,
                tabs,
                tab: session.tab,
                focus: WorkbenchFocus::Files,
                output: WorkbenchOutput::Build,
                panel_zoom: false,
                terminal: None,
                terminal_daemon: None,
                show_whitespace: session.show_whitespace,
                debug: session.debug,
            });
        }
        Ok(state)
    }
}
impl SavedWorkbenches {
    pub fn validate(&self) -> Result<(), String> {
        let mut bytes = 0usize;
        let mut ids = std::collections::HashSet::new();
        if self.sessions.len() > WORKBENCH_MAX_SESSIONS {
            return Err("Too many Workbench sessions".into());
        }
        for s in &self.sessions {
            if s.id == 0
                || !ids.insert(s.id)
                || s.name.trim().is_empty()
                || s.name.len() > 128
                || s.name.chars().any(char::is_control)
            {
                return Err("Invalid Workbench session identity".into());
            }
            if s.folders.len() > WORKBENCH_MAX_FOLDERS
                || s.tabs.len() > WORKBENCH_MAX_TABS
                || s.folder > s.folders.len().saturating_sub(1)
                || s.tab > s.tabs.len().saturating_sub(1)
                || s.tabs.iter().map(|t| t.breakpoints.len()).sum::<usize>()
                    > WORKBENCH_MAX_BREAKPOINTS
            {
                return Err("Invalid Workbench selection or size".into());
            }
            if s.folders.iter().any(|p| {
                !p.is_absolute()
                    || p.components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
            }) {
                return Err("Workbench folders must be absolute normalized paths".into());
            }
            if s.debug
                .fields()
                .iter()
                .any(|v| v.len() > 4096 || v.chars().any(char::is_control))
            {
                return Err("Invalid Workbench debug settings".into());
            }
            for t in &s.tabs {
                let line_count = t.text.bytes().filter(|b| *b == b'\n').count() + 1;
                if !s.folders.contains(&t.root)
                    || !t.path.starts_with(&t.root)
                    || t.path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                    || t.text.len() > TEXTAREA_MAX_BYTES
                    || t.original.len() > TEXTAREA_MAX_BYTES
                    || t.breakpoints.len() > WORKBENCH_MAX_BREAKPOINTS
                    || t.breakpoints
                        .iter()
                        .any(|line| *line == 0 || *line > line_count)
                {
                    return Err("Invalid Workbench file recovery".into());
                }
                bytes = bytes.saturating_add(t.text.len().max(t.original.len()).saturating_mul(2));
            }
        }
        if bytes > WORKBENCH_MAX_TEXT {
            return Err("Workbench recovery exceeds the 8 MiB text limit".into());
        }
        Ok(())
    }
}
