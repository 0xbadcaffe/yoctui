//! Semantic style projection shared by workspaces and shell chrome.

use super::*;

/// A selection is retained when focus moves; only its input owner gets a fill.
pub(super) fn selection_style(app: &App, selected: bool, focused: bool) -> Style {
    if selected {
        PaneShell::new("", false, pane_styles(app)).row_style(true, focused)
    } else {
        Style::default()
    }
}

pub(super) fn background_input_suspended(app: &App) -> bool {
    matches!(
        app.input_owner(),
        FocusTarget::Dialog | FocusTarget::CommandPalette
    )
}

pub(super) fn pane_has_focus(app: &App, owner: FocusTarget) -> bool {
    app.input_owner() == owner
}

pub(super) fn selected_style(app: &App, selected: bool) -> Style {
    selection_style(app, selected, pane_has_focus(app, FocusTarget::Workspace))
}

/// Called only by the visible dialog/palette that owns the input route.
pub(super) fn dialog_selected_style(app: &App, selected: bool) -> Style {
    selection_style(app, selected, true)
}

pub(super) fn selected_log_style(app: &App, severity: Severity) -> Style {
    let palette = ThemePalette::for_app(app);
    let style = severity_style(app, severity);
    if !pane_has_focus(app, FocusTarget::Workspace) {
        style.add_modifier(Modifier::UNDERLINED)
    } else if palette.attribute_only {
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
        BuildStatus::Cancelled | BuildStatus::Lost => palette.role(palette.warning, Modifier::BOLD),
        BuildStatus::Failed => palette.role(palette.error, Modifier::BOLD | Modifier::UNDERLINED),
        BuildStatus::LoadingWorkspace
        | BuildStatus::Parsing
        | BuildStatus::Running
        | BuildStatus::Cancelling => palette.role(palette.running, Modifier::BOLD),
        BuildStatus::Idle => palette.role(palette.disabled, Modifier::DIM),
    }
}
