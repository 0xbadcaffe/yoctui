//! Terminal workspace.
use super::*;

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
    let tabs = app
        .daemon
        .pty_sessions
        .iter()
        .enumerate()
        .flat_map(|(index, session)| {
            [
                Span::styled(
                    format!(" {}:{} ", session.id, session.name),
                    if index == app.pty_selection {
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
                .title("Terminal Sessions"),
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

pub(crate) fn terminal_session_panes(frame: &mut Frame, app: &App, area: Rect) {
    let layout = &app.pane_layout;
    let mut panes = Vec::new();
    collect_terminal_panes(&layout.root, area, &mut panes);
    let pane_count = panes.len();
    for (index, (rect, id)) in panes.into_iter().enumerate() {
        let session_index = if pane_count == 1 {
            app.pty_selection
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
        let block = pane_block(app, &title, session_index == app.pty_selection);
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

pub(crate) fn collect_terminal_panes(
    node: &PaneNode,
    area: Rect,
    output: &mut Vec<(Rect, yoctui_model::PaneId)>,
) {
    match node {
        PaneNode::Leaf { id } => output.push((area, *id)),
        PaneNode::Split {
            axis,
            ratio_per_mille,
            first,
            second,
        } => {
            let ratio = f32::from(*ratio_per_mille) / 1000.0;
            let (first_area, second_area) = match axis {
                SplitAxis::Horizontal => {
                    let first_width = (f32::from(area.width) * ratio) as u16;
                    let first_width = first_width.max(1).min(area.width.saturating_sub(1));
                    (
                        Rect {
                            width: first_width,
                            ..area
                        },
                        Rect {
                            x: area.x + first_width,
                            width: area.width - first_width,
                            ..area
                        },
                    )
                }
                SplitAxis::Vertical => {
                    let first_height = (f32::from(area.height) * ratio) as u16;
                    let first_height = first_height.max(1).min(area.height.saturating_sub(1));
                    (
                        Rect {
                            height: first_height,
                            ..area
                        },
                        Rect {
                            y: area.y + first_height,
                            height: area.height - first_height,
                            ..area
                        },
                    )
                }
            };
            collect_terminal_panes(first, first_area, output);
            collect_terminal_panes(second, second_area, output);
        }
    }
}

pub(crate) fn pane_switcher(frame: &mut Frame, app: &App, area: Rect) {
    let label = |target: FocusTarget, name: &str| {
        if app.focus == target {
            format!("[{name}]")
        } else {
            name.to_owned()
        }
    };
    let panes = yoctui_model::pane_focus_targets(app)
        .map(|target| label(target, target.label()))
        .collect::<Vec<_>>()
        .join("  ");
    frame.render_widget(
        Paragraph::new(format!("Panes: {panes}  Tab/Shift+Tab"))
            .style(ThemePalette::for_app(app).focus()),
        area,
    );
}

pub(crate) fn pane_block<'a>(app: &App, title: &'a str, focused: bool) -> Block<'a> {
    let palette = ThemePalette::for_app(app);
    PaneShell::new(
        Line::styled(title, palette.role(palette.informational, Modifier::BOLD)),
        focused,
        pane_styles(app),
    )
    .block()
}

pub(crate) fn pane_styles(app: &App) -> PaneStyles {
    let palette = ThemePalette::for_app(app);
    PaneStyles {
        base: palette.base(),
        border: Style::default().fg(palette.inactive_border),
        focused_border: palette.focus(),
        selected: palette.selected(),
        inactive_selected: if palette.attribute_only {
            Style::default().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::default().fg(palette.selection_foreground)
        },
        table_header: palette.role(palette.table_header, Modifier::BOLD),
        muted: palette.role(palette.muted, Modifier::DIM),
    }
}
