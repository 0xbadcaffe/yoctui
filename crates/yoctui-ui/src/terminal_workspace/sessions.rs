pub(crate) const TERMINAL_KEYBOARD_HELP: &str = "Ctrl+B t  open Terminal Sessions\nCtrl+B c  create build shell\nCtrl+B n/p  next/previous session\nCtrl+B % / \"  horizontal/vertical split\nCtrl+B z  zoom focused pane\nCtrl+B x  close pane (process keeps running)\nCtrl+B d  detach client (process keeps running)\nCtrl+B o/O  take/release writer control\nCtrl+B [  copy mode · Ctrl+B / search\nCtrl+B r  rename · Ctrl+B K confirmed kill\nCtrl+B Ctrl+B  send literal Ctrl+B\n\nWriter mode forwards ordinary characters and terminal keys verbatim. Paste always pauses for bounded review. Viewers can use the visible direct shortcuts because they cannot send PTY input.\n\nOnly the current daemon writer lease can send keyboard, paste, mouse, or resize input. Retained and remote-writer replicas remain read-only.";

pub(crate) fn terminal_sessions_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    if app.daemon.pty_sessions.is_empty() {
        if app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Help {
            frame.render_widget(
                Paragraph::new(TERMINAL_KEYBOARD_HELP)
                    .block(pane_block(app, "Terminal keyboard help", true))
                    .wrap(Wrap { trim: false }),
                area,
            );
            return;
        }
        let state = match app.daemon.status {
            yoctui_model::ClientReplicaStatus::Current => {
                "No daemon terminal sessions are active.\n\nCtrl+B c creates a build shell. Context actions can open devshell, menuconfig, SDK, Devtool, and Raw sessions."
            }
            yoctui_model::ClientReplicaStatus::Synchronizing => {
                "Synchronizing terminal sessions with the daemon…"
            }
            yoctui_model::ClientReplicaStatus::Stale => {
                "Terminal session state is stale. Reconnect before creating or controlling a process."
            }
            yoctui_model::ClientReplicaStatus::Disconnected => {
                "Daemon disconnected. Retained terminal identities are unavailable until reconnect."
            }
        };
        frame.render_widget(
            Paragraph::new(state)
                .block(pane_block(
                    app,
                    "Terminal Sessions · daemon-owned",
                    app.focus == FocusTarget::Workspace,
                ))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    let prefix_help_height = if app.selected_terminal_is_menuconfig() {
        0
    } else if area.height >= 30 {
        3
    } else {
        0
    };
    let regions = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(prefix_help_height),
    ])
    .split(area);
    let selected_index = app.selected_terminal_index();
    let embedded = app.embedded_platform_terminal_label();
    let tabs = app
        .daemon
        .pty_sessions
        .iter()
        .enumerate()
        .filter(|(index, _)| embedded.is_none() || Some(*index) == selected_index)
        .flat_map(|(index, session)| {
            [
                Span::styled(
                    format!(" {}:{} ", session.id, session.name),
                    if Some(index) == selected_index {
                        palette.selected()
                    } else {
                        palette.base()
                    },
                ),
                Span::raw(" │ "),
            ]
        })
        .collect::<Vec<_>>();
    let terminal_edge_border = if app.selected_terminal_is_menuconfig() {
        Borders::RIGHT
    } else {
        Borders::NONE
    };
    frame.render_widget(
        Paragraph::new(Line::from(tabs)).block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM | terminal_edge_border)
                .title(embedded.unwrap_or("Terminal Sessions")),
        ),
        regions[0],
    );
    if regions[3].height > 0 {
        frame.render_widget(Paragraph::new("Ctrl+B then: %/\" split · z zoom · [ copy · / search · d detach · x close · ? help")
            .block(pane_block(app,"Prefix help",false)),regions[3]);
    }

    let selected = app.selected_terminal_session();
    let screen = app.selected_terminal_screen();
    let details = app.selected_terminal_details();
    let access = selected.map_or("unavailable", |_| {
        if app.daemon.status != yoctui_model::ClientReplicaStatus::Current {
            return "RETAINED READ-ONLY · reconnect for control";
        }
        if app.selected_terminal_is_writer() {
            "WRITER"
        } else if details.is_some_and(|details| details.writer.is_some()) {
            "READ-ONLY · writer held by another client"
        } else {
            "VIEWER · writer available (o takes control)"
        }
    });
    let hits = if app.terminal.query.is_empty() {
        0
    } else {
        screen.map_or(0, |screen| {
            let query = app.terminal.query.to_lowercase();
            screen
                .rows
                .iter()
                .filter(|row| row.to_lowercase().contains(&query))
                .count()
        })
    };
    let replica = match app.daemon.status {
        yoctui_model::ClientReplicaStatus::Current => "CURRENT",
        yoctui_model::ClientReplicaStatus::Synchronizing => "RECONNECTING",
        yoctui_model::ClientReplicaStatus::Stale => "STALE",
        yoctui_model::ClientReplicaStatus::Disconnected => "DISCONNECTED",
    };
    let mode = match app.terminal.mode {
        yoctui_model::TerminalWorkbenchMode::Live => {
            let mut status = format!(
                "{replica} · LIVE · {access} · scrollback {}",
                app.terminal.scrollback_offset
            );
            if let Some(dropped) = screen
                .map(|screen| screen.dropped_line_feeds_lower_bound)
                .filter(|dropped| *dropped > 0)
            {
                status.push_str(&format!(" · dropped line-feeds ≥ {dropped}"));
            }
            status
        }
        yoctui_model::TerminalWorkbenchMode::Copy => format!(
            "COPY · row {} · ↑/↓ select · y/Enter copy · Esc live",
            app.terminal.copy_row.saturating_add(1)
        ),
        yoctui_model::TerminalWorkbenchMode::Search => format!(
            "SEARCH · {}▏ · {hits} visible match(es) · Enter/Esc finish",
            if app.terminal.query.is_empty() {
                "<empty>"
            } else {
                app.terminal.query.as_str()
            }
        ),
        yoctui_model::TerminalWorkbenchMode::Rename => {
            format!(
                "RENAME · {}▏ · Enter save · Esc cancel",
                app.terminal.rename
            )
        }
        yoctui_model::TerminalWorkbenchMode::PasteReview => format!(
            "PASTE REVIEW · {} bytes / {} line(s) · Enter send to writer · Esc cancel",
            app.terminal.pending_paste.len(),
            app.terminal
                .pending_paste
                .iter()
                .filter(|byte| **byte == b'\n')
                .count()
                + 1
        ),
        yoctui_model::TerminalWorkbenchMode::KillConfirmation => {
            "KILL CONFIRMATION · terminate the selected process group? Enter confirm · Esc cancel"
                .into()
        }
        yoctui_model::TerminalWorkbenchMode::Help => {
            "PREFIX HELP · Esc/? close · terminal input is paused while help is open".into()
        }
    };
    frame.render_widget(
        Paragraph::new(bounded_cell_text(&mode, regions[1].width.saturating_sub(1)))
            .style(match app.terminal.mode {
                yoctui_model::TerminalWorkbenchMode::KillConfirmation
                | yoctui_model::TerminalWorkbenchMode::PasteReview => {
                    palette.role(palette.warning, Modifier::BOLD)
                }
                _ => palette.role(palette.informational, Modifier::BOLD),
            })
            .block(Block::default().borders(terminal_edge_border)),
        regions[1],
    );

    if app.terminal.mode == yoctui_model::TerminalWorkbenchMode::Help {
        frame.render_widget(
            Paragraph::new(TERMINAL_KEYBOARD_HELP)
                .block(pane_block(app, "Terminal keyboard help", true))
                .wrap(Wrap { trim: false }),
            regions[2],
        );
    } else {
        terminal_session_panes(frame, app, regions[2]);
    }
}
