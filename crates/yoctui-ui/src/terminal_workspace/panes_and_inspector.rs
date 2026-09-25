pub(crate) fn terminal_session_panes(frame: &mut Frame, app: &App, area: Rect) {
    let layout = &app.pane_layout;
    let selected_index = app.selected_terminal_index().unwrap_or(app.pty_selection);
    let mut panes = Vec::new();
    if app.platform_menuconfig_visible() {
        panes.push((area, layout.focused));
    } else {
        collect_terminal_panes(&layout.root, area, &mut panes);
    }
    let pane_count = panes.len();
    for (index, (rect, id)) in panes.into_iter().enumerate() {
        let session_index = if pane_count == 1 {
            selected_index
        } else {
            index
        };
        let session = app.daemon.pty_sessions.get(session_index);
        let title = session
            .map(|session| format!(" {} #{} {:?} ", session.name, id.0, session.lifecycle))
            .unwrap_or_else(|| format!(" pane #{} ", id.0));
        let screen = session.and_then(|session| {
            app.daemon
                .pty_screens
                .iter()
                .find(|screen| screen.session_id == session.id)
        });
        let details = session.and_then(|session| {
            app.daemon
                .pty_details
                .iter()
                .find(|details| details.id == session.id)
        });
        let block = pane_block(app, &title, session_index == selected_index);
        let inner = block.inner(rect);
        frame.render_widget(block, rect);
        let status = session.map_or_else(
            || "No PTY session".into(),
            |session| {
                let access = if app.daemon.status != yoctui_model::ClientReplicaStatus::Current {
                    "retained read-only"
                } else if app.terminal.client_id.is_some()
                    && details.is_some_and(|details| details.writer == app.terminal.client_id)
                {
                    "writer"
                } else if details.is_some_and(|details| details.writer.is_some()) {
                    "read-only"
                } else {
                    "viewer"
                };
                format!(
                    "{:?} · {access} · {} viewer(s) · {}x{} · {}",
                    details.map_or(yoctui_model::ClientDaemonPtyKind::Utility, |details| {
                        details.kind
                    }),
                    session.viewers,
                    details.map_or(0, |details| details.columns),
                    details.map_or(0, |details| details.rows),
                    bounded_status_line(
                        details.map_or_else(String::new, |details| details.cwd.clone()),
                        32
                    )
                )
            },
        );
        let mut status_lines = vec![Line::raw(bounded_cell_text(&status, inner.width))];
        if let Some(screen) = screen {
            if !app.terminal.query.is_empty() {
                let query = app.terminal.query.to_lowercase();
                let hits = screen
                    .rows
                    .iter()
                    .filter(|row| row.to_lowercase().contains(&query))
                    .count();
                status_lines.push(Line::raw(bounded_cell_text(
                    &format!("last search: {} · {hits} matches", app.terminal.query),
                    inner.width,
                )));
            }
            if screen.scrollback_lines > 0 || screen.dropped_line_feeds_lower_bound > 0 {
                status_lines.push(Line::raw(bounded_cell_text(
                    &format!(
                        "history {} · dropped ≥{}",
                        screen.scrollback_lines, screen.dropped_line_feeds_lower_bound
                    ),
                    inner.width,
                )));
            }
        }
        let regions = Layout::vertical([
            Constraint::Length(status_lines.len().min(u16::MAX as usize) as u16),
            Constraint::Min(0),
        ])
        .split(inner);
        frame.render_widget(Paragraph::new(status_lines), regions[0]);
        let rendered = screen.is_some_and(|screen| {
            render_terminal_replica_content(frame, app, screen, regions[1], 0, 0)
        });
        if !rendered {
            let lines = screen.map_or_else(
                || vec![Line::raw("Screen unavailable · awaiting daemon snapshot")],
                |screen| {
                    screen
                        .rows
                        .iter()
                        .map(|row| Line::raw(row.as_str()))
                        .collect()
                },
            );
            frame.render_widget(Paragraph::new(lines), regions[1]);
        }
    }
}

pub(crate) fn terminal_session_inspector_text(app: &App) -> String {
    let Some(session) = app.selected_terminal_session() else {
        return format!(
            "Replica: {:?}\n\nNo terminal session is selected.",
            app.daemon.status
        );
    };
    let details = app.selected_terminal_details();
    let role = if app.selected_terminal_is_writer() {
        "Writer (keyboard/paste/resize enabled)"
    } else if details.is_some_and(|details| details.writer.is_some()) {
        "Read-only viewer (another client owns writer)"
    } else {
        "Viewer (writer lease available)"
    };
    let writer = details.and_then(|details| details.writer).map_or_else(
        || "none".into(),
        |id| {
            id.iter()
                .take(4)
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        },
    );
    let mut lines = vec![
        format!("Session: {} ({})", session.name, session.id),
        format!(
            "Identity: {:?}",
            details.map_or(yoctui_model::ClientDaemonPtyKind::Utility, |details| {
                details.kind
            })
        ),
        format!("Lifecycle: {:?}", session.lifecycle),
        format!("Role: {role}"),
        format!(
            "Writer: {writer} · epoch {}",
            details.map_or(0, |details| details.writer_epoch)
        ),
        format!("Viewers: {}", session.viewers),
        format!(
            "Dimensions: {}x{}",
            details.map_or(0, |details| details.columns),
            details.map_or(0, |details| details.rows)
        ),
        format!(
            "Working directory: {}",
            details.map_or("unavailable", |details| details.cwd.as_str())
        ),
        format!(
            "Restartable: {}",
            if details.is_some_and(|details| details.restartable) {
                "yes"
            } else {
                "no"
            }
        ),
        format!(
            "Exit code: {}",
            details
                .and_then(|details| details.exit_code)
                .map_or_else(|| "unavailable".into(), |code| code.to_string())
        ),
    ];
    if let Some(screen) = app.selected_terminal_screen() {
        lines.extend([
            String::new(),
            "Replica / history".into(),
            format!("Viewport offset: {}", screen.scrollback_offset),
            format!("Retained scrollback: {} lines", screen.scrollback_lines),
            if screen.dropped_line_feeds_lower_bound == 0 {
                "Dropped history: none observed".into()
            } else {
                format!(
                    "Dropped history: at least {} observed line-feed(s)",
                    screen.dropped_line_feeds_lower_bound
                )
            },
            format!(
                "Cursor: {},{} ({})",
                screen.cursor_column,
                screen.cursor_row,
                if screen.cursor_hidden {
                    "hidden"
                } else {
                    "visible"
                }
            ),
        ]);
    } else {
        lines.extend([
            String::new(),
            "Screen replica unavailable; process identity and lifecycle remain authoritative."
                .into(),
        ]);
    }
    if matches!(session.lifecycle, yoctui_model::ClientDaemonLifecycle::Lost) {
        lines.extend([
            String::new(),
            "Lost means the daemon cannot prove a live process remains attached. No automatic restart is implied."
                .into(),
        ]);
    }
    lines.join("\n")
}
