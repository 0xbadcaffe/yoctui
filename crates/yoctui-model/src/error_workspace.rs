//! Current and archived build diagnostics with a bounded source-log viewer.
use crate::{
    App, Dialog, Effect, LogEntry, SavedBuild, SavedBuildLog, SavedBuildOutcome, Severity,
    close_dialog, open_dialog,
};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ErrorWorkspaceView {
    #[default]
    Current,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorLogViewer {
    pub title: String,
    pub path: Option<PathBuf>,
    pub content: String,
    pub scroll: usize,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ErrorWorkspaceState {
    pub view: ErrorWorkspaceView,
    pub history_selection: usize,
    pub viewer: Option<ErrorLogViewer>,
}

#[derive(Debug, Clone, Copy)]
pub struct HistoricalError<'a> {
    pub build: &'a SavedBuild,
    pub log: &'a SavedBuildLog,
    pub resolved: bool,
}

pub fn saved_build_is_resolved(records: &[SavedBuild], build: &SavedBuild) -> bool {
    matches!(build.outcome, SavedBuildOutcome::Failed)
        && build.machine.is_some()
        && records.iter().any(|candidate| {
            candidate.saved_unix_ms > build.saved_unix_ms
                && candidate.target == build.target
                && candidate.machine == build.machine
                && candidate.outcome == SavedBuildOutcome::Succeeded
        })
}

pub fn historical_errors(records: &[SavedBuild]) -> Vec<HistoricalError<'_>> {
    records
        .iter()
        .flat_map(|build| {
            let resolved = saved_build_is_resolved(records, build);
            build
                .logs
                .iter()
                .filter(|log| matches!(log.severity, Severity::Warning | Severity::Error))
                .map(move |log| HistoricalError {
                    build,
                    log,
                    resolved,
                })
        })
        .collect()
}

pub fn selected_historical_error(app: &App) -> Option<HistoricalError<'_>> {
    historical_errors(&app.saved_builds.records)
        .get(app.error_workspace.history_selection)
        .copied()
}

pub fn selected_error_path(app: &App) -> Option<PathBuf> {
    match app.error_workspace.view {
        ErrorWorkspaceView::Current => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .and_then(|entry| entry.path.clone()),
        ErrorWorkspaceView::History => selected_historical_error(app)
            .and_then(|entry| entry.log.path.as_deref())
            .map(PathBuf::from),
    }
}

fn current_fallback(entry: &LogEntry) -> String {
    let mut text = entry.message.clone();
    if let Some(info) = &entry.diagnostic {
        text.push_str("\n\nCategory: ");
        text.push_str(&info.category);
        for suggestion in &info.suggestions {
            text.push_str("\nSuggested: ");
            text.push_str(suggestion);
        }
    }
    text
}

fn selected_viewer_source(app: &App) -> Option<(String, Option<PathBuf>, String)> {
    match app.error_workspace.view {
        ErrorWorkspaceView::Current => {
            app.logs
                .diagnostics()
                .nth(app.error_selection)
                .map(|entry| {
                    (
                        entry.path.as_deref().map_or_else(
                            || "Retained diagnostic".into(),
                            |path| path.display().to_string(),
                        ),
                        entry.path.clone(),
                        current_fallback(entry),
                    )
                })
        }
        ErrorWorkspaceView::History => selected_historical_error(app).map(|entry| {
            let path = entry.log.path.as_deref().map(PathBuf::from);
            (
                path.as_deref().map_or_else(
                    || format!("Saved error · {}", entry.build.target),
                    |path| path.display().to_string(),
                ),
                path,
                entry.log.message.clone(),
            )
        }),
    }
}

pub(crate) fn reduce_error_workspace(app: &mut App, action: crate::Action) -> Option<Effect> {
    use crate::Action;
    match action {
        Action::SetErrorWorkspaceView(view) => {
            app.error_workspace.view = view;
            app.error_workspace.viewer = None;
        }
        Action::SelectHistoricalError { delta } => {
            let count = historical_errors(&app.saved_builds.records).len();
            app.error_workspace.history_selection = app
                .error_workspace
                .history_selection
                .saturating_add_signed(delta)
                .min(count.saturating_sub(1));
        }
        Action::OpenSelectedErrorLog => {
            let Some((title, path, fallback)) = selected_viewer_source(app) else {
                app.notification = Some("No build diagnostic is selected.".into());
                return None;
            };
            let loading = path.is_some();
            app.error_workspace.viewer = Some(ErrorLogViewer {
                title,
                path: path.clone(),
                content: fallback,
                scroll: 0,
                loading,
                error: None,
            });
            if let Some(path) = path {
                return Some(Effect::LoadErrorLog(path));
            }
        }
        Action::ErrorLogLoaded { path, content } => {
            if let Some(viewer) = app.error_workspace.viewer.as_mut()
                && viewer.path.as_deref() == Some(path.as_path())
            {
                viewer.content = content;
                viewer.loading = false;
                viewer.error = None;
                viewer.scroll = 0;
            }
        }
        Action::ErrorLogLoadFailed { path, message } => {
            if let Some(viewer) = app.error_workspace.viewer.as_mut()
                && viewer.path.as_deref() == Some(path.as_path())
            {
                viewer.loading = false;
                viewer.error = Some(message);
            }
        }
        Action::ScrollErrorLog { delta } => {
            if let Some(viewer) = app.error_workspace.viewer.as_mut() {
                let lines = viewer.content.lines().count().max(1);
                viewer.scroll = viewer
                    .scroll
                    .saturating_add_signed(delta)
                    .min(lines.saturating_sub(1));
            }
        }
        Action::CloseErrorLog => app.error_workspace.viewer = None,
        Action::RequestResolvedBuildRemoval => {
            let Some(selected) = selected_historical_error(app) else {
                app.notification = Some("No saved build error is selected.".into());
                return None;
            };
            if !selected.resolved {
                app.notification = Some(
                    "This saved failure is unresolved; only resolved failures can be removed."
                        .into(),
                );
                return None;
            }
            open_dialog(
                app,
                Dialog::ResolvedBuildRemovalConfirmation {
                    id: selected.build.id.clone(),
                    target: selected.build.target.clone(),
                },
            );
        }
        Action::ConfirmResolvedBuildRemoval => {
            let Some(Dialog::ResolvedBuildRemovalConfirmation { id, .. }) =
                app.active_dialog().cloned()
            else {
                return None;
            };
            close_dialog(app);
            return Some(Effect::RemoveSavedBuild(id));
        }
        Action::CancelResolvedBuildRemoval => close_dialog(app),
        Action::ResolvedBuildRemoved { id } => {
            let mut records = app.saved_builds.records.as_ref().clone();
            records.retain(|record| record.id != id);
            app.saved_builds.records = std::sync::Arc::new(records);
            let count = historical_errors(&app.saved_builds.records).len();
            app.error_workspace.history_selection = app
                .error_workspace
                .history_selection
                .min(count.saturating_sub(1));
            app.notification = Some("Removed the resolved build from Yoctui history.".into());
        }
        Action::ResolvedBuildRemovalFailed { message } => {
            app.notification = Some(format!("Could not remove saved build history: {message}"));
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    None
}
