use super::*;

pub(crate) fn daemon_manager_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.daemon_manager;
    let palette = ThemePalette::for_app(app);
    let sections = Layout::vertical([Constraint::Length(5), Constraint::Min(1)]).split(area);
    let busy = app
        .daemon
        .jobs
        .iter()
        .filter(|job| !job.lifecycle.is_terminal())
        .count();
    let sessions = app
        .daemon
        .pty_sessions
        .iter()
        .filter(|session| !session.lifecycle.is_terminal())
        .count();
    let summary = format!(
        "1 Local daemon · IPC {:?} · systemd {}\nJobs {busy} · running terminals {sessions} · clients {} · sequence {}\n1 Health / activity   2 Daemon logs · {}",
        app.daemon.status,
        if state.service_active {
            "active"
        } else {
            "inactive / unavailable"
        },
        app.daemon.connected_clients,
        app.daemon.sequence,
        state.message.as_deref().unwrap_or(if state.loading {
            "Refreshing…"
        } else {
            "Ready"
        })
    );
    frame.render_widget(
        Paragraph::new(summary).block(pane_block(
            app,
            "Daemons · local",
            app.focus == FocusTarget::Workspace,
        )),
        sections[0],
    );
    let lines = app.daemon_manager_lines();
    let capacity = usize::from(sections[1].height.saturating_sub(2)).max(1);
    let offset = state.scroll.min(lines.len().saturating_sub(capacity));
    frame.render_widget(
        Paragraph::new(lines.join("\n"))
            .scroll((offset.min(u16::MAX as usize) as u16, 0))
            .block(pane_block(
                app,
                if state.logs_visible {
                    "Daemon logs · latest 200 journal lines · r refresh"
                } else {
                    "Service / health / activity"
                },
                app.focus == FocusTarget::Workspace,
            )),
        sections[1],
    );
    if state.editing || state.review.is_some() {
        let width = area.width.saturating_sub(4).min(100);
        let height = area.height.saturating_sub(2).min(10);
        let popup = Rect::new(
            area.x + (area.width - width) / 2,
            area.y + (area.height - height) / 2,
            width,
            height,
        );
        let text = if let Some(action) = state.review {
            format!(
                "{} local yoctui.service?\nBuild: {}\nSource: {}\nClients disconnect on stop/restart; active work blocks this action.\nConfiguration overrides the launch command using a persistent drop-in.\nBuild files stay untouched.\nEnter confirm · Esc cancel",
                action.label(),
                state.build_directory,
                state.source_directory
            )
        } else {
            format!(
                "{} Build: {}\n{} Source (contains oe-init-build-env): {}\nTab field · Ctrl+U clear · paste supported · Enter review · Esc cancel",
                if state.source_field { " " } else { "▶" },
                state.build_directory,
                if state.source_field { "▶" } else { " " },
                state.source_directory
            )
        };
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(text)
                .wrap(Wrap { trim: false })
                .style(Style::default().fg(palette.warning))
                .block(pane_block(
                    app,
                    if state.editing {
                        "Configure local daemon"
                    } else {
                        "Daemon control review"
                    },
                    true,
                )),
            popup,
        );
    }
}
