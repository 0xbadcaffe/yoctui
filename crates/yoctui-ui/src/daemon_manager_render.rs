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
        Paragraph::new(
            lines
                .iter()
                .flat_map(|line| {
                    line.split('\n')
                        .map(|line| styled_daemon_line(line, state.logs_visible, &palette))
                })
                .collect::<Vec<_>>(),
        )
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

pub(crate) fn styled_daemon_line<'a>(
    text: &'a str,
    journal: bool,
    palette: &ThemePalette,
) -> Line<'a> {
    if journal {
        return styled_journal_line(text, palette);
    }
    for (prefix, color) in [
        ("Recovery: ", palette.warning),
        ("Recent activity: ", palette.informational),
        ("Current work: ", palette.running),
        ("Working: ", palette.running),
        ("Terminal: ", palette.running),
    ] {
        if let Some(body) = text.strip_prefix(prefix) {
            return Line::from(vec![
                Span::styled(prefix, palette.role(color, Modifier::BOLD)),
                Span::styled(
                    body,
                    palette.role(
                        if prefix == "Recovery: " {
                            palette.warning
                        } else {
                            daemon_message_color(body, palette)
                        },
                        Modifier::empty(),
                    ),
                ),
            ]);
        }
    }
    Line::raw(text)
}

fn styled_journal_line<'a>(text: &'a str, palette: &ThemePalette) -> Line<'a> {
    if let Some((timestamp, rest)) = text.split_once(' ')
        && timestamp.len() >= 19
        && timestamp.as_bytes().get(4) == Some(&b'-')
        && timestamp.as_bytes().get(10) == Some(&b'T')
        && let Some((host, rest)) = rest.split_once(' ')
        && !host.is_empty()
        && let Some((process, message)) = rest.split_once(": ")
        && !process.is_empty()
    {
        return Line::from(vec![
            Span::styled(
                timestamp,
                palette.role(palette.secondary_foreground, Modifier::empty()),
            ),
            Span::raw(" "),
            Span::styled(host, palette.role(palette.informational, Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(process, palette.role(palette.accent, Modifier::BOLD)),
            Span::styled(
                ": ",
                palette.role(palette.secondary_foreground, Modifier::empty()),
            ),
            Span::styled(
                message,
                palette.role(daemon_message_color(message, palette), Modifier::empty()),
            ),
        ]);
    }
    Line::styled(
        text,
        palette.role(daemon_message_color(text, palette), Modifier::empty()),
    )
}

fn daemon_message_color(text: &str, palette: &ThemePalette) -> Color {
    let has = |tokens: &[&str]| {
        text.split(|c: char| !c.is_ascii_alphabetic())
            .any(|word| tokens.iter().any(|token| word.eq_ignore_ascii_case(token)))
    };
    if has(&["error", "fatal", "failed", "panic", "critical"]) {
        palette.error
    } else if has(&["warning", "warn"]) {
        palette.warning
    } else if has(&["debug", "trace"]) {
        palette.secondary_foreground
    } else if has(&["ready", "succeeded", "successfully"]) {
        palette.success
    } else {
        palette.primary_foreground
    }
}
