//! Client-local, read-only comparison with independent layer-style file trees.
use crate::*;
mod algorithm;
pub use algorithm::compare_file_text;

pub const FILE_DIFF_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const FILE_DIFF_MAX_LINES: usize = 20_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffPane {
    pub browser: LayerBrowser,
    pub marked: Option<PathBuf>,
    pub request: u64,
    pub loading: bool,
    pub error: Option<String>,
    pub offset: usize,
}
impl Default for FileDiffPane {
    fn default() -> Self {
        Self {
            browser: LayerBrowser::new("Diff".into(), PathBuf::new()),
            marked: None,
            request: 0,
            loading: false,
            error: None,
            offset: 0,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FileDiffState {
    pub initialized: bool,
    pub panes: [FileDiffPane; 2],
    pub active: usize,
    pub generation: u64,
    pub comparing: bool,
    pub comparison: Option<FileDiffComparison>,
    pub selection: usize,
    pub offset: usize,
    pub visible_rows: usize,
    pub horizontal: usize,
    pub error: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffCell {
    pub number: usize,
    pub text: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffRow {
    pub left: Option<FileDiffCell>,
    pub right: Option<FileDiffCell>,
    pub equal: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileDiffComparison {
    pub paths: [PathBuf; 2],
    pub rows: Vec<FileDiffRow>,
    pub identical: bool,
    pub final_newline: [bool; 2],
    pub changed_rows: usize,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDiffRequest {
    Directory {
        pane: usize,
        request: u64,
        directory: PathBuf,
    },
    Compare {
        generation: u64,
        paths: [PathBuf; 2],
    },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileDiffAction {
    Initialize(PathBuf),
    SwitchPane,
    Select {
        delta: isize,
    },
    SelectAt {
        pane: usize,
        index: usize,
    },
    Expand,
    ToggleDirectory,
    Collapse,
    Parent,
    ToggleHidden,
    Refresh,
    Mark,
    Compare,
    Horizontal {
        delta: isize,
    },
    Back,
    DirectoryLoaded {
        pane: usize,
        request: u64,
        directory: PathBuf,
        result: Result<Vec<LayerBrowserEntry>, String>,
    },
    Compared {
        generation: u64,
        result: Result<FileDiffComparison, String>,
    },
}

fn directory_request(state: &mut FileDiffState, pane: usize, directory: PathBuf) -> Effect {
    let side = &mut state.panes[pane];
    side.request = side.request.wrapping_add(1);
    side.loading = true;
    side.error = None;
    Effect::FileDiff(FileDiffRequest::Directory {
        pane,
        request: side.request,
        directory,
    })
}

/// Initial scans are scheduled independently by the runtime (including offline).
pub fn file_diff_initial_request(app: &mut App, root: PathBuf) -> Option<Effect> {
    if !app.file_diff.initialized {
        let _ = reduce(app, FileDiffAction::Initialize(root));
    }
    let pane = app
        .file_diff
        .panes
        .iter()
        .position(|pane| pane.browser.nodes.is_empty() && pane.request == 0)?;
    let directory = app.file_diff.panes[pane].browser.root.clone();
    Some(directory_request(&mut app.file_diff, pane, directory))
}

pub(crate) fn reduce(app: &mut App, action: FileDiffAction) -> Option<Effect> {
    let state = &mut app.file_diff;
    match action {
        FileDiffAction::Initialize(root) if !state.initialized => {
            state.initialized = true;
            for pane in &mut state.panes {
                pane.browser = LayerBrowser::new("Diff".into(), root.clone());
            }
        }
        FileDiffAction::Initialize(_) => {}
        FileDiffAction::DirectoryLoaded {
            pane,
            request,
            directory,
            result,
        } => {
            let side = state.panes.get_mut(pane)?;
            if side.request != request || !side.loading {
                return None;
            }
            side.loading = false;
            match result {
                Ok(entries) => {
                    let preferred = side.browser.selected_entry().map(|e| e.path.clone());
                    side.browser.nodes.insert(directory, entries);
                    side.browser.rebuild(preferred.as_ref());
                }
                Err(message) => side.error = Some(message),
            }
        }
        FileDiffAction::Compared { generation, result } => {
            if state.generation != generation || !state.comparing {
                return None;
            }
            state.comparing = false;
            match result {
                Ok(comparison) => {
                    state.comparison = Some(comparison);
                    state.selection = 0;
                    state.horizontal = 0;
                }
                Err(message) => state.error = Some(message),
            }
        }
        FileDiffAction::Back => {
            if state.comparison.take().is_some() || state.comparing {
                state.comparing = false;
                state.generation = state.generation.wrapping_add(1);
                state.error = None;
            } else {
                app.focus = FocusTarget::Navigator;
                if app.zoomed_pane.is_some() {
                    app.zoomed_pane = Some(app.focus);
                }
            }
        }
        FileDiffAction::SwitchPane => state.active = 1 - state.active,
        FileDiffAction::SelectAt { pane, index } if pane < 2 => {
            state.active = pane;
            app.focus = FocusTarget::Workspace;
            if let Some(comparison) = &state.comparison {
                state.selection = index.min(comparison.rows.len().saturating_sub(1));
            } else {
                state.panes[pane].browser.selection =
                    index.min(state.panes[pane].browser.entries.len().saturating_sub(1));
            }
        }
        FileDiffAction::SelectAt { .. } => {}
        FileDiffAction::Select { delta } => {
            if let Some(comparison) = &state.comparison {
                state.selection = shifted_index(state.selection, delta, comparison.rows.len());
            } else {
                let browser = &mut state.panes[state.active].browser;
                browser.selection = shifted_index(browser.selection, delta, browser.entries.len());
            }
        }
        FileDiffAction::Horizontal { delta } if state.comparison.is_some() => {
            state.horizontal = shifted_index(state.horizontal, delta, FILE_DIFF_MAX_BYTES);
        }
        FileDiffAction::Mark if state.comparison.is_none() && !state.comparing => {
            let side = &mut state.panes[state.active];
            if let Some(entry) = side.browser.selected_entry().filter(|e| !e.is_dir) {
                side.marked = if side.marked.as_ref() == Some(&entry.path) {
                    None
                } else {
                    Some(entry.path.clone())
                };
                state.error = None;
            } else {
                state.error = Some("Select a regular text file in each pane with Space.".into());
            }
        }
        FileDiffAction::Compare if !state.comparing => {
            let [Some(left), Some(right)] = state.panes.each_ref().map(|pane| pane.marked.clone())
            else {
                state.error = Some("Mark one file on each side with Space, then press F5.".into());
                return None;
            };
            state.generation = state.generation.wrapping_add(1);
            state.comparing = true;
            state.error = None;
            return Some(Effect::FileDiff(FileDiffRequest::Compare {
                generation: state.generation,
                paths: [left, right],
            }));
        }
        action @ (FileDiffAction::Expand | FileDiffAction::ToggleDirectory)
            if state.comparison.is_none() && !state.comparing =>
        {
            let pane = state.active;
            let side = &mut state.panes[pane];
            if side.loading {
                return None;
            }
            let entry = side.browser.selected_entry()?.clone();
            if !entry.is_dir {
                return None;
            }
            if side.browser.expanded.insert(entry.path.clone()) {
                if !side.browser.nodes.contains_key(&entry.path) {
                    return Some(directory_request(state, pane, entry.path));
                }
            } else if matches!(action, FileDiffAction::ToggleDirectory) {
                side.browser.expanded.remove(&entry.path);
            }
            side.browser.rebuild(Some(&entry.path));
        }
        FileDiffAction::Collapse if state.comparison.is_none() && !state.comparing => {
            let side = &mut state.panes[state.active];
            let entry = side.browser.selected_entry()?.clone();
            if side.browser.expanded.remove(&entry.path) {
                side.browser.rebuild(Some(&entry.path));
            } else if let Some(parent) = entry.path.parent().filter(|p| *p != side.browser.root) {
                let parent = parent.to_owned();
                side.browser.expanded.remove(&parent);
                side.browser.rebuild(Some(&parent));
            } else {
                return reduce(app, FileDiffAction::Parent);
            }
        }
        FileDiffAction::Parent if state.comparison.is_none() && !state.comparing => {
            let pane = state.active;
            if state.panes[pane].loading {
                return None;
            }
            let parent = state.panes[pane].browser.root.parent()?.to_owned();
            let hidden = state.panes[pane].browser.show_hidden;
            state.panes[pane].browser = LayerBrowser::new("Diff".into(), parent.clone());
            state.panes[pane].browser.show_hidden = hidden;
            return Some(directory_request(state, pane, parent));
        }
        FileDiffAction::ToggleHidden if state.comparison.is_none() => {
            let browser = &mut state.panes[state.active].browser;
            let preferred = browser.selected_entry().map(|e| e.path.clone());
            browser.show_hidden = !browser.show_hidden;
            browser.rebuild(preferred.as_ref());
        }
        FileDiffAction::Refresh if state.comparison.is_none() => {
            let pane = state.active;
            let directory = state.panes[pane]
                .browser
                .selected_entry()
                .map(|e| {
                    e.path
                        .parent()
                        .unwrap_or(&state.panes[pane].browser.root)
                        .to_owned()
                })
                .unwrap_or_else(|| state.panes[pane].browser.root.clone());
            return Some(directory_request(state, pane, directory));
        }
        _ => {}
    }
    state.reconcile_viewport(state.visible_rows);
    None
}

impl FileDiffState {
    pub fn reconcile_viewport(&mut self, rows: usize) {
        self.visible_rows = rows;
        for side in &mut self.panes {
            side.offset = BoundedScroll::new(
                side.browser.selection,
                side.offset,
                rows,
                side.browser.entries.len(),
            )
            .offset;
        }
        if let Some(diff) = &self.comparison {
            self.offset =
                BoundedScroll::new(self.selection, self.offset, rows + 1, diff.rows.len()).offset;
        }
    }
}

#[cfg(test)]
#[path = "tests/file_diff.rs"]
mod tests;
