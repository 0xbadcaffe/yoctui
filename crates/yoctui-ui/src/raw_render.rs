//! Raw render.
use super::*;

pub(crate) fn raw_mode_workspace(frame: &mut Frame, app: &App, area: Rect, terminal_width: u16) {
    use yoctui_model::{RawBrowserColumn, RawModeView};

    if app.raw_mode.view == RawModeView::Execution {
        raw_execution_workspace(frame, app, area, terminal_width);
        return;
    }
    if app.raw_mode.view == RawModeView::Favorites {
        raw_favorites_workspace(frame, app, area, terminal_width);
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

    let columns = Layout::horizontal([Constraint::Percentage(52), Constraint::Min(20)]).split(area);
    raw_category_browser(frame, app, columns[0]);
    raw_command_list(frame, app, columns[1]);
}

pub(crate) fn raw_favorites_workspace(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    terminal_width: u16,
) {
    use yoctui_model::RawModeFocus;
    let catalog = yoctui_model::builtin_raw_catalog();
    let authority = app.workspace_compatibility.authority();
    let selected = app.raw_mode.favorite_selection;
    let count = app.raw_mode.favorites.len();
    let viewport_height = usize::from(area.height.saturating_sub(4)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        (count > 0).then_some(selected),
        count,
        viewport_height,
    );
    let position = BoundedScrollIndicator::new(viewport.start, viewport.len(), count).label();
    let active =
        app.focus == FocusTarget::Workspace && app.raw_mode.focus == RawModeFocus::Favorites;
    let title = format!("Raw Mode / Favorites · {position}");
    let block = pane_block(app, &title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    if app.raw_mode.favorites.is_empty() {
        frame.render_widget(
            Paragraph::new("No Raw favorites.\n\nf adds the selected command from the catalog.\nEsc returns to the catalog browser.")
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    }
    let width = usize::from(inner.width);
    let mut lines = Vec::new();
    for (offset, favorite) in app.raw_mode.favorites[viewport.clone()].iter().enumerate() {
        let index = viewport.start + offset;
        let selected_row = index == selected;
        let projection = favorite.project(catalog, authority);
        let state = raw_availability_label(projection.availability.state);
        let stale = if projection.stale { " · STALE" } else { "" };
        let command = catalog
            .command(&favorite.command)
            .map(raw_command_template)
            .unwrap_or_else(|| "<command absent from current catalog>".into());
        let prefix = if selected_row { "▶ " } else { "  " };
        lines.push(Line::raw(bounded_cell_text(
            &format!("{prefix}{} · {state}{stale}", favorite.name),
            width as u16,
        )));
        lines.push(Line::raw(bounded_cell_text(
            &format!("    {command}"),
            width as u16,
        )));
        let defaults = if favorite.parameter_defaults.is_empty() {
            "defaults: --"
        } else {
            "defaults: configured"
        };
        let argv = if favorite.additional_arguments.as_slice().is_empty() {
            "argv: --".to_owned()
        } else {
            format!(
                "argv: {}",
                favorite.additional_arguments.as_slice().join(" ")
            )
        };
        lines.push(Line::raw(bounded_cell_text(
            &format!("    {defaults} · {argv}"),
            width as u16,
        )));
        lines.push(Line::raw(bounded_cell_text(
            &format!(
                "    reason: {}",
                projection
                    .reason
                    .as_deref()
                    .unwrap_or("Current catalog and capability authority")
            ),
            width as u16,
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
    let footer = if terminal_width < 100 {
        "Enter reopen · i inspect · r rename · o reorder · x remove · Esc back"
    } else {
        "Enter reopen · i inspect · r rename · ↑/↓ reorder · x remove · Esc back"
    };
    if inner.height > 1 {
        let footer_area = Rect {
            x: inner.x,
            y: inner.y + inner.height.saturating_sub(1),
            width: inner.width,
            height: 1,
        };
        frame.render_widget(Paragraph::new(footer), footer_area);
    }
}

pub(crate) fn raw_execution_workspace(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    terminal_width: u16,
) {
    use yoctui_model::{
        RawAttachmentState, RawExecutionOutcome, RawExecutionOwner, RawExecutionPhase,
        RawInteractionMode, RawOutputStream,
    };

    let Some(execution) = app.raw_mode.selected_execution() else {
        let command = app
            .raw_mode
            .execution
            .as_ref()
            .map_or("unknown", |command| command.as_str());
        frame.render_widget(
            Paragraph::new(format!(
                "Command: {command}\nState: Awaiting daemon acknowledgement\n\nNo local process exists. The execution view will install the validated daemon replica when it arrives."
            ))
            .block(pane_block(app, "Raw execution · connecting", true))
            .wrap(Wrap { trim: false }),
            area,
        );
        return;
    };

    let phase = match execution.phase {
        RawExecutionPhase::Queued => "QUEUED",
        RawExecutionPhase::Starting => "STARTING",
        RawExecutionPhase::Running => "RUNNING",
        RawExecutionPhase::Cancelling => "CANCELLING",
        RawExecutionPhase::Terminal(RawExecutionOutcome::Succeeded) => "SUCCEEDED",
        RawExecutionPhase::Terminal(RawExecutionOutcome::Failed) => "FAILED",
        RawExecutionPhase::Terminal(RawExecutionOutcome::Cancelled) => "CANCELLED",
        RawExecutionPhase::Terminal(RawExecutionOutcome::Lost) => "LOST",
    };
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
    let rows = Layout::vertical([
        Constraint::Length(if terminal_width >= 100 { 7 } else { 9 }),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(vec![
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
        ])
        .block(pane_block(app, "Raw execution", true))
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
        Paragraph::new(if terminal_width < 100 {
            "c Cancel  d Detach  r Reattach  f Follow  / Search  Esc Back"
        } else {
            "c Cancel · d Detach · r Reattach · f Follow · / Search · Esc Back · ↑/↓ Scroll · ←/→ Horizontal · 1 stdout · 2 stderr"
        })
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
    let viewport = usize::from(area.height.saturating_sub(2)).max(1);
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
        .map(|line| Line::raw(line.chars().skip(horizontal).collect::<String>()))
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
            "retained {} B/{} lines · dropped {} B/{} lines · truncated {} · hits {hits}",
            output.retained_bytes,
            output.retained_lines,
            output.dropped_bytes,
            output.dropped_lines,
            output.truncated_chunks
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
        true,
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
        || " Raw PTY · awaiting daemon session ".into(),
        |session| {
            let dimensions = screen.map_or_else(
                || "size unavailable".into(),
                |screen| format!("{}x{}", screen.columns, screen.rows_count),
            );
            format!(
                " Raw PTY · {:?} · {dimensions} · daemon writer lease · {} viewer(s) ",
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
