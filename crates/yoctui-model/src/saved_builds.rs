//! Read-only build history, kept separate from live execution authority.
use crate::{App, FocusTarget, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SavedBuildOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Lost,
    Incomplete,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedBuildLog {
    pub unix_ms: u64,
    pub severity: Severity,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedBuildTask {
    pub recipe: String,
    pub task: String,
    pub status: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedBuild {
    pub id: String,
    pub target: String,
    pub machine: Option<String>,
    pub source: Option<String>,
    pub build_dir: Option<String>,
    pub outcome: SavedBuildOutcome,
    pub saved_unix_ms: u64,
    pub started_unix_ms: Option<u64>,
    pub finished_unix_ms: Option<u64>,
    pub logs: Vec<SavedBuildLog>,
    pub tasks: Vec<SavedBuildTask>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavedBuildView {
    Summary,
    Logs,
    Tasks,
    Errors,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SavedBuildState {
    pub records: std::sync::Arc<Vec<SavedBuild>>,
    pub selection: usize,
    pub view: Option<SavedBuildView>,
    pub scroll: usize,
    pub browsing: bool,
    pub loading: bool,
    pub reload_requested: bool,
    pub notice: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SavedBuildAction {
    Loaded {
        records: Vec<SavedBuild>,
        notice: Option<String>,
    },
    Select(isize),
    Open,
    Close,
    ShiftView(isize),
    Scroll(isize),
    Toggle,
    Refresh,
}
pub(crate) fn reduce_saved_build(app: &mut App, action: SavedBuildAction) {
    let offline = app.is_offline();
    let state = &mut app.saved_builds;
    match action {
        SavedBuildAction::Loaded { records, notice } => {
            let previous = state.records.get(state.selection).map(|r| r.id.clone());
            let initial = state.records.is_empty();
            state.records = std::sync::Arc::new(records);
            state.selection = previous
                .and_then(|id| state.records.iter().position(|r| r.id == id))
                .unwrap_or(0);
            state.loading = false;
            state.notice = notice;
            if initial && !state.records.is_empty() {
                state.browsing = true;
            }
            if state.records.is_empty() {
                state.view = None;
            }
        }
        SavedBuildAction::Select(delta) => {
            state.selection = state
                .selection
                .saturating_add_signed(delta)
                .min(state.records.len().saturating_sub(1));
            state.scroll = 0;
        }
        SavedBuildAction::Open if !state.records.is_empty() => {
            state.view = Some(SavedBuildView::Summary);
            state.scroll = 0;
            app.focus = FocusTarget::Workspace;
        }
        SavedBuildAction::Open => {}
        SavedBuildAction::Close => {
            state.view = None;
            state.scroll = 0;
        }
        SavedBuildAction::ShiftView(delta) => {
            let views = [
                SavedBuildView::Summary,
                SavedBuildView::Logs,
                SavedBuildView::Tasks,
                SavedBuildView::Errors,
            ];
            let index = state
                .view
                .and_then(|v| views.iter().position(|x| *x == v))
                .unwrap_or(0);
            state.view = Some(views[(index as isize + delta).rem_euclid(4) as usize]);
            state.scroll = 0;
        }
        SavedBuildAction::Scroll(delta) => {
            state.scroll = state.scroll.saturating_add_signed(delta).min(4096);
        }
        SavedBuildAction::Toggle if !offline => {
            state.browsing = !state.browsing;
            state.view = None;
        }
        SavedBuildAction::Toggle => {}
        SavedBuildAction::Refresh => {
            state.reload_requested = true;
        }
    }
}

#[cfg(test)]
#[path = "tests/saved_builds/mod.rs"]
mod tests;
