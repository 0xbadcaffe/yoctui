pub(crate) fn terminal_session_browser(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let indexes = app.terminal_browser_indexes();
    let block = pane_block(
        app,
        "Terminal Sessions · session tree",
        app.focus == FocusTarget::Workspace,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let reviewing_removal = app.terminal.mode == yoctui_model::TerminalWorkbenchMode::KillConfirmation;
    let regions = Layout::vertical([Constraint::Length(if reviewing_removal { 3 } else { 1 }), Constraint::Min(0)]).split(inner);
    let heading = if app.terminal.mode == yoctui_model::TerminalWorkbenchMode::KillConfirmation {
        app.terminal.kill_target.as_ref().map_or_else(
            || "Remove session: target unavailable · Esc cancel".into(),
            |target| format!("Enter remove · Esc cancel\nTERM → KILL after 2s\n#{} {}", target.session_id, target.name),
        )
    } else if app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Rename {
        format!("Rename: {}▏ · Enter save · Esc cancel", app.terminal.rename)
    } else {
        format!(
            "▾ Daemon terminals ({}) · Enter open · n new shell · Del/x remove",
            indexes.len()
        )
    };
    frame.render_widget(
        Paragraph::new(heading).style(palette.role(palette.informational, Modifier::BOLD)),
        regions[0],
    );
    if indexes.is_empty() {
        let message = if app.daemon.status == yoctui_model::ClientReplicaStatus::Current {
            "No available sessions. Ctrl+B c creates a build shell. Exited sessions are hidden; daemon history is preserved."
        } else {
            "Daemon unavailable or reconnecting. Session control requires a current connection."
        };
        frame.render_widget(
            Paragraph::new(message).wrap(Wrap { trim: false }),
            regions[1],
        );
        return;
    }
    let selected = indexes
        .iter()
        .position(|index| Some(*index) == app.selected_terminal_index())
        .unwrap_or(0);
    let range = yoctui_model::centered_viewport_range(
        Some(selected),
        indexes.len(),
        usize::from(regions[1].height).max(1),
    );
    let items = range
        .clone()
        .map(|row| {
            let session = &app.daemon.pty_sessions[indexes[row]];
            let details = app
                .daemon
                .pty_details
                .iter()
                .find(|details| details.id == session.id);
            let branch = if row + 1 == indexes.len() {
                "└─"
            } else {
                "├─"
            };
            let access = if app.daemon.status != yoctui_model::ClientReplicaStatus::Current {
                "reconnect"
            } else if details.is_some_and(|details| {
                details.writer.is_some() && details.writer != app.terminal.client_id
            }) {
                "viewer · writer held"
            } else {
                "available"
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!(" {branch} #{} {} ", session.id, session.name),
                    palette.base(),
                ),
                Span::styled(
                    format!("{:?} · {access}", session.lifecycle),
                    palette.role(palette.informational, Modifier::empty()),
                ),
                Span::styled(
                    details.map_or(String::new(), |details| {
                        format!(" · {:?} · {}", details.kind, details.cwd)
                    }),
                    palette.base(),
                ),
            ]))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default().with_selected(Some(selected.saturating_sub(range.start)));
    frame.render_stateful_widget(
        List::new(items)
            .highlight_style(palette.selected())
            .highlight_symbol("▶ "),
        regions[1],
        &mut state,
    );
}
