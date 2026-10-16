//! Raw render.
use super::*;

pub(crate) fn raw_mode_workspace(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    use yoctui_model::{RawBrowserColumn, RawModeView};
    let rows = Layout::vertical([
        Constraint::Length(u16::from(app.raw_mode.notification.is_some())),
        Constraint::Length(u16::from(
            app.raw_mode.view == RawModeView::Browser
                && (app.raw_mode.search.editing || !app.raw_mode.search.query.is_empty()),
        )),
        Constraint::Min(0),
    ])
    .split(area);
    if let Some(message) = &app.raw_mode.notification {
        frame.render_widget(
            Paragraph::new(bounded_cell_text(message, rows[0].width)).style(
                ThemePalette::for_app(app).role(ThemePalette::for_app(app).warning, Modifier::BOLD),
            ),
            rows[0],
        );
    }
    if rows[1].height > 0 {
        frame.render_widget(
            Paragraph::new(bounded_cell_text(
                &format!(
                    "Command filter: {}{} · Enter {} · Ctrl+U clear",
                    app.raw_mode.search.query,
                    if app.raw_mode.search.editing {
                        "▏"
                    } else {
                        ""
                    },
                    if app.raw_mode.search.editing {
                        "finish"
                    } else {
                        "review"
                    }
                ),
                rows[1].width,
            )),
            rows[1],
        );
    }
    let area = rows[2];

    if app.raw_mode.view == RawModeView::Execution {
        raw_execution_workspace(frame, app, area, terminal_width);
        return;
    }
    if app.raw_mode.view == RawModeView::Favorites {
        raw_favorites_workspace(frame, app, area, terminal_width);
        return;
    }
    if app.raw_mode.view == RawModeView::History {
        raw_history_workspace(frame, app, area);
        return;
    }
    if app.raw_mode.view != RawModeView::Browser {
        frame.render_widget(
            Paragraph::new("Raw Mode retained view\n\nEsc returns to the catalog browser.")
                .block(pane_block(app, "Raw Mode", false))
                .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }

    if terminal_width < 100 {
        match app.raw_mode.browser_column {
            RawBrowserColumn::Categories => raw_category_browser(frame, app, area),
            RawBrowserColumn::Commands => raw_command_list(frame, app, area),
        }
        return;
    }

    let columns = Layout::horizontal([
        Constraint::Length(yoctui_model::raw_category_column_width(area.width)),
        Constraint::Min(0),
    ])
    .split(area);
    raw_category_browser(frame, app, columns[0]);
    raw_command_list(frame, app, columns[1]);
}

pub(crate) fn raw_execution_workspace(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    terminal_width: u16,
) {
    use yoctui_model::{
        RawAttachmentState, RawExecutionOwner, RawInteractionMode, RawOutputStream,
    };

    let Some(execution) = app.raw_mode.selected_execution() else {
        let command = app
            .raw_mode
            .execution
            .as_ref()
            .map_or("unknown", |command| command.as_str());
        frame.render_widget(
            Paragraph::new(format!(
                "Command: {command}\nState: Awaiting daemon acknowledgement\n\nNo local process exists. The execution view will install the validated daemon replica when it arrives.\n\nEsc returns to the catalog; accepted daemon work continues."
            ))
            .block(pane_block(app, "Raw execution · connecting", true))
            .wrap(Wrap { trim: false }),
            area,
        );
        return;
    };

    let phase = crate::raw_management::raw_phase_label(execution.phase);
    let attachment = match execution.attachment {
        RawAttachmentState::Attached => "ATTACHED",
        RawAttachmentState::Detached => "DETACHED",
    };
    let interaction = match execution.request.interaction {
        RawInteractionMode::NoninteractiveJob => "Noninteractive job",
        RawInteractionMode::InteractivePty => "Interactive PTY",
    };
    let owner = execution
        .owner
        .as_ref()
        .map_or("pending".into(), |owner| match owner {
            RawExecutionOwner::Job(id) => id.to_string(),
            RawExecutionOwner::Pty(id) => id.to_string(),
        });
    let result = execution.result.as_ref().map_or_else(
        || "Result: pending".into(),
        |result| {
            format!(
                "Result: {:?} · exit {}{}",
                result.outcome,
                result
                    .exit_code
                    .map_or_else(|| "--".into(), |code| code.to_string()),
                result
                    .message
                    .as_ref()
                    .map_or_else(String::new, |message| format!(" · {message}"))
            )
        },
    );
    let compact = area.height < 22;
    let rows = Layout::vertical([
        Constraint::Length(if compact {
            5
        } else if terminal_width >= 100 {
            7
        } else {
            9
        }),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);
    let status = if compact {
        vec![
            Line::raw(format!("{phase} · {attachment} · {interaction}")),
            Line::raw(result),
            Line::raw(format!(
                "Search: {} · Follow: {}",
                app.raw_mode.output.query,
                if app.raw_mode.output.follow {
                    "ON"
                } else {
                    "PAUSED"
                }
            )),
        ]
    } else {
        vec![
            Line::raw(format!(
                "{} · {} · {} · {}",
                execution.request.command, interaction, phase, attachment
            )),
            Line::raw(format!("Request: {}", execution.request.id)),
            Line::raw(format!(
                "Owner: {owner} · elapsed {} ms · capability generation {}",
                execution.elapsed_ms, execution.request.capability_generation
            )),
            Line::raw(result),
            Line::raw(format!(
                "Search: {} · Follow: {}",
                if app.raw_mode.output.query.is_empty() {
                    "--"
                } else {
                    app.raw_mode.output.query.as_str()
                },
                if app.raw_mode.output.follow {
                    "ON"
                } else {
                    "PAUSED"
                }
            )),
        ]
    };
    frame.render_widget(
        Paragraph::new(status)
            .block(pane_block(
                app,
                &if compact {
                    format!("Raw execution · {}", execution.request.command)
                } else {
                    "Raw execution".into()
                },
                true,
            ))
            .wrap(Wrap { trim: false }),
        rows[0],
    );

    match execution.request.interaction {
        RawInteractionMode::NoninteractiveJob => {
            if terminal_width >= 110 {
                let streams =
                    Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                        .split(rows[1]);
                raw_output_stream(
                    frame,
                    app,
                    &execution.stdout,
                    RawOutputStream::Stdout,
                    streams[0],
                );
                raw_output_stream(
                    frame,
                    app,
                    &execution.stderr,
                    RawOutputStream::Stderr,
                    streams[1],
                );
            } else {
                let retained = match app.raw_mode.output.stream {
                    RawOutputStream::Stdout => &execution.stdout,
                    RawOutputStream::Stderr => &execution.stderr,
                };
                raw_output_stream(frame, app, retained, app.raw_mode.output.stream, rows[1]);
            }
        }
        RawInteractionMode::InteractivePty => raw_pty_output(frame, app, execution, rows[1]),
    }
    frame.render_widget(
        Paragraph::new(crate::footer::raw_mode_shortcuts(app))
            .block(Block::default().borders(Borders::TOP)),
        rows[2],
    );
}

pub(crate) fn raw_output_stream(
    frame: &mut Frame,
    app: &App,
    output: &yoctui_model::RawRetainedOutput,
    stream: yoctui_model::RawOutputStream,
    area: Rect,
) {
    let label = match stream {
        yoctui_model::RawOutputStream::Stdout => "stdout",
        yoctui_model::RawOutputStream::Stderr => "stderr",
    };
    let lines = output
        .chunks
        .iter()
        .flat_map(|chunk| {
            let lines = chunk.text.lines().collect::<Vec<_>>();
            if lines.is_empty() { vec![""] } else { lines }
        })
        .collect::<Vec<_>>();
    let viewport = usize::from(area.height.saturating_sub(3)).max(1);
    let start = if app.raw_mode.output.follow {
        lines.len().saturating_sub(viewport)
    } else {
        lines
            .len()
            .saturating_sub(viewport)
            .saturating_sub(app.raw_mode.output.vertical_scroll)
    };
    let horizontal = app.raw_mode.output.horizontal_scroll;
    let mut visible = lines
        .iter()
        .skip(start)
        .take(viewport)
        .map(|line| {
            let text = line.chars().skip(horizontal).collect::<String>();
            if !app.raw_mode.output.query.is_empty()
                && line
                    .to_lowercase()
                    .contains(&app.raw_mode.output.query.to_lowercase())
            {
                Line::styled(
                    text,
                    ThemePalette::for_app(app)
                        .role(ThemePalette::for_app(app).warning, Modifier::BOLD),
                )
            } else {
                Line::raw(text)
            }
        })
        .collect::<Vec<_>>();
    let hits = if app.raw_mode.output.query.is_empty() {
        0
    } else {
        lines
            .iter()
            .filter(|line| {
                line.to_lowercase()
                    .contains(&app.raw_mode.output.query.to_lowercase())
            })
            .count()
    };
    visible.insert(
        0,
        Line::raw(format!(
            "hits {hits} · dropped {}B/{}L · trunc {}",
            output.dropped_bytes, output.dropped_lines, output.truncated_chunks
        )),
    );
    let title = format!(
        " {label} · {} B/{} lines ",
        output.retained_bytes, output.retained_lines,
    );
    yocto_logs::render(
        frame,
        area,
        Text::from(if visible.is_empty() {
            vec![Line::raw("No retained output")]
        } else {
            visible
        }),
        pane_block(app, &title, app.raw_mode.output.stream == stream),
        false,
    );
}

pub(crate) fn raw_pty_output(
    frame: &mut Frame,
    app: &App,
    execution: &yoctui_model::RawExecutionState,
    area: Rect,
) {
    let pty_id = match execution.owner.as_ref() {
        Some(yoctui_model::RawExecutionOwner::Pty(session)) => session.daemon_pty_id(),
        _ => None,
    };
    let session = pty_id.and_then(|id| app.daemon.pty_sessions.iter().find(|pty| pty.id == id));
    let screen = pty_id.and_then(|id| {
        app.daemon
            .pty_screens
            .iter()
            .find(|pty| pty.session_id == id)
    });
    let title = session.map_or_else(
        || " Raw PTY preview · o opens interactive terminal ".into(),
        |session| {
            let dimensions = screen.map_or_else(
                || "size unavailable".into(),
                |screen| format!("{}x{}", screen.columns, screen.rows_count),
            );
            format!(
                " Raw PTY preview · o terminal · {:?} · {dimensions} · {} viewer(s) ",
                session.lifecycle, session.viewers
            )
        },
    );
    if let Some(screen) = screen {
        let block = pane_block(app, &title, true);
        let inner = block.inner(area);
        let viewport = usize::from(inner.height).max(1);
        let row_offset = if app.raw_mode.output.follow {
            usize::from(screen.rows_count).saturating_sub(viewport)
        } else {
            usize::from(screen.rows_count)
                .saturating_sub(viewport)
                .saturating_sub(app.raw_mode.output.vertical_scroll)
        };
        frame.render_widget(block, area);
        if render_terminal_replica_content(
            frame,
            app,
            screen,
            inner,
            row_offset,
            app.raw_mode.output.horizontal_scroll,
        ) {
            return;
        }
        let plain_row_offset = if app.raw_mode.output.follow {
            screen.rows.len().saturating_sub(viewport)
        } else {
            screen
                .rows
                .len()
                .saturating_sub(viewport)
                .saturating_sub(app.raw_mode.output.vertical_scroll)
        };
        let lines = screen
            .rows
            .iter()
            .skip(plain_row_offset)
            .take(viewport)
            .map(|line| {
                Line::raw(
                    line.chars()
                        .skip(app.raw_mode.output.horizontal_scroll)
                        .collect::<String>(),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }
    let lines = screen.map_or_else(
        || vec![Line::raw("Screen unavailable · awaiting daemon snapshot")],
        |screen| {
            let viewport = usize::from(area.height.saturating_sub(2)).max(1);
            let start = if app.raw_mode.output.follow {
                screen.rows.len().saturating_sub(viewport)
            } else {
                screen
                    .rows
                    .len()
                    .saturating_sub(viewport)
                    .saturating_sub(app.raw_mode.output.vertical_scroll)
            };
            screen
                .rows
                .iter()
                .skip(start)
                .take(viewport)
                .map(|line| {
                    Line::raw(
                        line.chars()
                            .skip(app.raw_mode.output.horizontal_scroll)
                            .collect::<String>(),
                    )
                })
                .collect()
        },
    );
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(app, &title, true))
            .wrap(Wrap { trim: false }),
        area,
    );
}
