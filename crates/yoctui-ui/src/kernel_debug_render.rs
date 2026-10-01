use super::*;
use yoctui_model::{KernelDebugOperation, KernelDebugTool, TerminalLaunchDestination};

pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let block = pane_block(app, "Kernel", app.focus == FocusTarget::Workspace);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let state = &app.kernel_debug;
    let mut lines = vec![
        Line::from(vec![
            Span::raw(" 1 Configuration  │  2 Device trees  │ "),
            Span::styled(" 3 Debugging ", selected_style(app, true)),
        ]),
        Line::from(
            state
                .error
                .as_deref()
                .unwrap_or(if state.pending.is_some() {
                    "Inspecting local tools…"
                } else {
                    "Tool presence only · target tools/permissions are checked when launched"
                }),
        ),
    ];
    let capacity = usize::from(area.height.saturating_sub(6)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        Some(state.selection),
        KernelDebugTool::ALL.len(),
        capacity,
    );
    for index in viewport {
        let tool = KernelDebugTool::ALL[index];
        let status = tool.program().map_or("Guide only", |program| {
            if state
                .tools
                .as_ref()
                .is_some_and(|tools| tools.programs.contains_key(program))
            {
                "Host found"
            } else {
                "Host missing"
            }
        });
        lines.push(Line::styled(
            format!(
                "{} {:<40} {status}",
                if index == state.selection { "▶" } else { " " },
                tool.label()
            ),
            selected_style(app, index == state.selection),
        ));
    }
    lines.push(Line::from(
        "Enter opens typed tool options or guide; no process starts here.",
    ));
    lines.push(Line::from(
        "strace traces userspace syscalls; GDB/KGDB debugs kernel source.",
    ));
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(crate) fn dialog(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if let Some(Dialog::KernelDebug(dialog)) = app.active_dialog() {
        let popup = bounded_dialog_rect(area, 110, 28);
        frame.render_widget(Clear, popup);
        let tool = dialog.draft.tool;
        let block = dialog_block(
            app,
            format!("Kernel debugging · {}", tool.label()),
            DialogTone::Standard,
        );
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        if tool.program().is_none() {
            let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(2)]).split(inner);
            frame.render_widget(
                Paragraph::new(format!(
                    "GUIDE ONLY · no command is launched\n\n{}\n\nReference: {}",
                    tool.guide(),
                    tool.reference()
                ))
                .wrap(Wrap { trim: false })
                .scroll((dialog.guide_scroll, 0)),
                rows[0],
            );
            frame.render_widget(Paragraph::new("↑/↓/PgUp/PgDn scroll · Esc close\nKernel settings, crash/reboot and privilege changes remain manual."), rows[1]);
        } else {
            let fields = dialog.draft.fields();
            let rows = Layout::vertical([
                Constraint::Length(2),
                Constraint::Length(fields.len().min(6) as u16),
                Constraint::Min(1),
                Constraint::Length(2),
                Constraint::Length(2),
            ])
            .split(inner);
            frame.render_widget(
                Paragraph::new(if tool.runtime_target() {
                    dialog
                        .draft
                        .value(yoctui_model::KernelDebugField::Destination)
                } else {
                    "HOST client/offline analysis · files must match target architecture/build"
                })
                .wrap(Wrap { trim: false }),
                rows[0],
            );
            let capacity = usize::from(rows[1].height).max(1);
            let viewport = yoctui_model::centered_viewport_range(
                Some(dialog.selection),
                fields.len(),
                capacity,
            );
            let lines = viewport
                .map(|index| {
                    Line::styled(
                        format!(
                            "{} {}: {}{}",
                            if index == dialog.selection {
                                "▶"
                            } else {
                                " "
                            },
                            fields[index].label(),
                            dialog.draft.value(fields[index]),
                            if index == dialog.selection { "_" } else { "" }
                        ),
                        selected_style(app, index == dialog.selection),
                    )
                })
                .collect::<Vec<_>>();
            frame.render_widget(Paragraph::new(lines), rows[1]);
            let status = dialog.error.as_deref().unwrap_or(
                if matches!(
                    app.kernel_debug.pending,
                    Some(KernelDebugOperation::Prepare { .. })
                ) {
                    "Validating executable and files…"
                } else {
                    "Attach/breakpoints can PAUSE execution; tracing adds overhead/sensitive data."
                },
            );
            frame.render_widget(
                Paragraph::new(format!("{}\nReference: {}", tool.guide(), tool.reference()))
                    .wrap(Wrap { trim: false })
                    .scroll((dialog.guide_scroll, 0)),
                rows[2],
            );
            frame.render_widget(Paragraph::new(status).wrap(Wrap { trim: false }), rows[3]);
            frame.render_widget(Paragraph::new("Tab/↑/↓ field · ←/→/Space scope · type · Ctrl+U clear · PgUp/Dn guide\nEnter review exact launch · Esc cancel without spawning").wrap(Wrap { trim: false }), rows[4]);
        }
        return true;
    }
    let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
        return false;
    };
    if Some(&dialog.request) != app.kernel_debug.prepared.as_ref() {
        return false;
    }
    let popup = bounded_dialog_rect(area, 110, 28);
    frame.render_widget(Clear, popup);
    let block = dialog_block(
        app,
        "Review Kernel debugging launch",
        DialogTone::Confirmation,
    );
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(5),
    ])
    .split(inner);
    frame.render_widget(Paragraph::new(format!("{}\nAttach/breakpoints can PAUSE the target; profiling adds overhead.\nMay expose sensitive data. No sudo, installation or kernel-setting changes.\nWorking directory: {}", dialog.request.name, dialog.request.cwd.display())).wrap(Wrap { trim: false }), rows[0]);
    let mut lines = vec![Line::from(format!(
        "Executable: {}",
        dialog.request.program.display()
    ))];
    lines.extend(
        dialog
            .request
            .arguments
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("argv[{index}]: {argument:?}"))),
    );
    frame.render_widget(
        Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((app.kernel_debug.preview_scroll, 0)),
        rows[1],
    );
    let detached = match &app.detached_terminal {
        yoctui_model::DetachedTerminalAvailability::Available { launcher } => launcher.clone(),
        yoctui_model::DetachedTerminalAvailability::Unavailable { reason } => {
            format!("unavailable: {reason}")
        }
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::styled(
                "Embedded in Yoctui (daemon-owned PTY)",
                selected_style(
                    app,
                    dialog.destination == TerminalLaunchDestination::Embedded,
                ),
            ),
            Line::styled(
                format!("Detached terminal · {detached}"),
                selected_style(
                    app,
                    dialog.destination == TerminalLaunchDestination::Detached,
                ),
            ),
            Line::from("↑/↓ destination · PgUp/PgDn scroll exact argv"),
            Line::from("Enter launches on the SHOWN scope · Esc cancels without spawning"),
        ])
        .wrap(Wrap { trim: false }),
        rows[2],
    );
    true
}
