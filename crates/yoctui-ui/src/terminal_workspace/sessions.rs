pub(crate) const TERMINAL_KEYBOARD_HELP: &str = concat!(
    "Terminal controls\n",
    "Alt+o  Back to sessions; the process keeps running\n",
    "Alt+/  Find output in this console (not global file search)\n",
    "Esc  Close this help and resume the console\n\n",
    "Ctrl+B is a prefix: press it, release it, then press the next key.\n",
    "Ctrl+B ?  Show this shortcut reference\n",
    "Ctrl+B c  New build shell\n",
    "Ctrl+B n / p  Next / previous session\n",
    "Ctrl+B [  Copy mode; arrows select a line, Enter copies, Esc returns\n",
    "Ctrl+B r  Rename this session\n",
    "Ctrl+B e  Return to the retained editor, when available\n",
    "Ctrl+B % / \"  Horizontal / vertical split\n",
    "Ctrl+B z  Zoom the focused pane\n",
    "Ctrl+B x  Close pane; the process keeps running\n",
    "Ctrl+B Alt+k  Review stopping this process\n",
    "Ctrl+B Ctrl+B  Send a literal Ctrl+B to the application\n\n",
    "When you are the writer, Tab, q and ordinary / go to the application.\n",
    "F1 help, F2 tasks and F12 menu remain Yoctui shortcuts, except in GitUI\n",
    "where function keys belong to GitUI. Alt+o is the reliable way back.\n",
    "A read-only console uses o to take control, if no other client owns it.\n\n",
    "Find output: type a query; arrows/PgUp/PgDn browse history, Home/End go\n",
    "to oldest/live output, Ctrl+U clears, Enter/Esc resumes the console.\n",
    "Matches count only the displayed page. Ctrl+B / is an alternative.\n",
    "Global search: use / in the session tree, not in an open console."
);

pub(crate) fn terminal_sessions_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    if app.terminal_browser_visible() && app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Help {
        terminal_session_browser(frame, app, area);
        return;
    }
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

    let regions = Layout::vertical([
        Constraint::Length(if app.platform_menuconfig_visible() { 3 } else { 0 }),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(0),
    ])
    .split(area);
    if let Some(label) = app.embedded_platform_terminal_label() {
        frame.render_widget(Paragraph::new(format!("{label} · INPUT: MENUCONFIG · Ctrl+G Yoctui"))
            .block(Block::default().borders(Borders::TOP | Borders::BOTTOM)), regions[0]);
    }
    let terminal_edge_border = if app.selected_terminal_uses_native_workspace() {
        Borders::RIGHT
    } else {
        Borders::NONE
    };

    let selected = app.selected_terminal_session();
    let screen = app.selected_terminal_screen();
    let details = app.selected_terminal_details();
    let access = selected.map_or("unavailable", |session| {
        if app.daemon.status != yoctui_model::ClientReplicaStatus::Current {
            return "RETAINED READ-ONLY · reconnect for control";
        }
        if session.lifecycle != yoctui_model::ClientDaemonLifecycle::Running {
            return "READ-ONLY · no active writer";
        }
        if details.is_some_and(|details|
            app.terminal.releasing_writer == Some((details.id, details.writer_epoch))) {
            return "VIEWER · waiting for writer release";
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
                "Terminal Sessions · {replica} · LIVE · {access} · scrollback {}",
                app.terminal.scrollback_offset
            );
            if let Some(dropped) = screen
                .map(|screen| screen.dropped_line_feeds_lower_bound)
                .filter(|dropped| *dropped > 0)
            {
                status.push_str(&format!(" · dropped line-feeds ≥ {dropped}"));
            }
            if !app.terminal.query.is_empty() {
                status.push_str(&format!(" · search: {} · {hits} visible match(es)", app.terminal.query));
            }
            status
        }
        yoctui_model::TerminalWorkbenchMode::Copy => format!(
            "COPY · row {} · ↑/↓ select · y/Enter copy · Esc live",
            app.terminal.copy_row.saturating_add(1)
        ),
        yoctui_model::TerminalWorkbenchMode::Search => format!(
            "SEARCH · {}▏ · {hits} visible match(es) · PgUp/PgDn history · Ctrl+U clear · Enter/Esc finish",
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
            app.terminal.kill_target.as_ref().map_or_else(
                || "KILL CONFIRMATION · target unavailable · Esc cancel".into(),
                |target| format!("KILL #{}:{} and remove? TERM → KILL after 2s · Enter confirm · Esc cancel", target.session_id, target.name),
            )
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
