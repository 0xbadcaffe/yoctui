//! Semantic style projection shared by workspaces and shell chrome.

use super::*;

pub(super) fn selected_style(app: &App, active: bool) -> Style {
    if active {
        PaneShell::new("", false, pane_styles(app)).row_style(true, true)
    } else {
        Style::default()
    }
}

pub(super) fn selected_log_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    let style = severity_style(app, severity);
    if palette.attribute_only {
        style.add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        style
            .bg(palette.selection_background)
            .add_modifier(Modifier::BOLD)
    }
}

pub(super) fn severity_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    match severity {
        Severity::Trace => palette.role(palette.disabled, Modifier::DIM),
        Severity::Info => palette.role(palette.informational, Modifier::ITALIC),
        Severity::Warning => palette.role(palette.warning, Modifier::BOLD),
        Severity::Error => palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
    }
}

pub(super) fn build_status_style(app: &App) -> Style {
    let palette = ThemePalette::for_app(app);
    match app.build.status {
        BuildStatus::Completed => palette.role(palette.success, Modifier::BOLD),
        BuildStatus::Cancelled => palette.role(palette.warning, Modifier::BOLD),
        BuildStatus::Failed => palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
        BuildStatus::LoadingWorkspace
        | BuildStatus::Parsing
        | BuildStatus::Running
        | BuildStatus::Cancelling => palette.role(palette.running, Modifier::BOLD),
        BuildStatus::Idle => palette.role(palette.disabled, Modifier::DIM),
    }
}
