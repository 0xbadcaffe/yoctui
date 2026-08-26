//! Responsive pane topology for wide, medium, and narrow terminals.

use super::*;

pub(super) fn responsive_shell(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    terminal_width: u16,
    now: SystemTime,
) {
    let task_rows = (app.screen == Screen::Tasks).then(|| app.visible_task_row_refs_at(now));
    let task_rows = task_rows.as_deref();
    if let Some(zoomed) = app.zoomed_pane {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        frame.render_widget(
            Paragraph::new(format!(
                "ZOOM · {:?} · {} · Esc restore",
                app.screen,
                app.zoom_label().unwrap_or("unknown pane")
            ))
            .style(
                ThemePalette::for_app(app).role(ThemePalette::for_app(app).accent, Modifier::BOLD),
            ),
            rows[0],
        );
        match zoomed {
            FocusTarget::Navigator => navigator(frame, app, rows[1], task_rows),
            FocusTarget::Workspace
                if app.screen == Screen::Dashboard && !app.daemon.pty_sessions.is_empty() =>
            {
                terminal_session_panes(frame, app, rows[1]);
            }
            FocusTarget::Workspace if app.screen == Screen::Signatures => {
                signatures_workspace(frame, app, rows[1], terminal_width);
            }
            FocusTarget::Workspace
                if app.screen == Screen::Tasks
                    && app.workspace_subfocus == yoctui_model::WorkspaceSubfocus::Context =>
            {
                render_tasks_context_zoom(frame, app, rows[1], now);
            }
            FocusTarget::Workspace => {
                workspace(frame, app, rows[1], terminal_width, now, task_rows);
            }
            FocusTarget::Inspector => inspector(frame, app, rows[1], now, task_rows),
            FocusTarget::Dialog | FocusTarget::CommandPalette => {
                workspace(frame, app, rows[1], terminal_width, now, task_rows);
            }
        }
        return;
    }
    if app.screen == Screen::Dashboard && !app.daemon.pty_sessions.is_empty() {
        terminal_session_panes(frame, app, area);
        return;
    }
    if app.screen == Screen::Signatures {
        signatures_workspace(frame, app, area, terminal_width);
        return;
    }
    if terminal_width >= WIDE_WORKBENCH_MIN_WIDTH {
        let panes = if app.screen == Screen::Tasks && terminal_width == 160 {
            Layout::horizontal([
                Constraint::Length(26),
                Constraint::Length(89),
                Constraint::Length(45),
            ])
            .split(area)
        } else if app.screen == Screen::Tasks {
            Layout::horizontal([
                Constraint::Length(22),
                Constraint::Percentage(56),
                Constraint::Min(32),
            ])
            .split(area)
        } else {
            Layout::horizontal([
                Constraint::Length(22),
                Constraint::Percentage(43),
                Constraint::Min(28),
            ])
            .split(area)
        };
        navigator(frame, app, panes[0], task_rows);
        workspace(frame, app, panes[1], terminal_width, now, task_rows);
        inspector(frame, app, panes[2], now, task_rows);
    } else if terminal_width >= 100 {
        let panes = Layout::horizontal([Constraint::Length(22), Constraint::Min(40)]).split(area);
        navigator(frame, app, panes[0], task_rows);
        workspace(frame, app, panes[1], terminal_width, now, task_rows);
        if app.focus == FocusTarget::Inspector {
            frame.render_widget(Clear, panes[1]);
            inspector(frame, app, panes[1], now, task_rows);
        }
    } else {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        pane_switcher(frame, app, rows[0]);
        match app.focus {
            FocusTarget::Navigator => navigator(frame, app, rows[1], task_rows),
            FocusTarget::Inspector => inspector(frame, app, rows[1], now, task_rows),
            FocusTarget::Workspace | FocusTarget::Dialog | FocusTarget::CommandPalette => {
                workspace(frame, app, rows[1], terminal_width, now, task_rows);
            }
        }
    }
}
