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
        let status = tool.program().map_or(
            if tool.configuration_prep() {
                "Config prep"
            } else {
                "Guide only"
            },
            |program| {
                if state
                    .tools
                    .as_ref()
                    .is_some_and(|tools| tools.programs.contains_key(program))
                    || (tool == KernelDebugTool::QemuGdb
                        && app
                            .workspace_compatibility
                            .authority()
                            .and_then(|authority| {
                                authority.snapshot.environment.available_tools.value()
                            })
                            .is_some_and(|tools| tools.iter().any(|tool| tool.id == "runqemu")))
                {
                    "Host found"
                } else {
                    "Host missing"
                }
            },
        );
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
        if tool.configuration_prep() {
            crate::kernel_instrumentation_render::dialog(frame, app, dialog, inner);
        } else if tool.program().is_none() {
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
                    if tool == KernelDebugTool::QemuGdb {
                        "MANAGED HOST GUEST · snapshot/nonetwork · Linux · not a physical target"
                    } else if tool == KernelDebugTool::KgdbSerial {
                        "HOST GDB → PHYSICAL BOARD · requires already halted/configured exclusive serial target"
                    } else {
                        "HOST client/offline analysis · files must match target architecture/build"
                    }
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
                } else if matches!(app.kernel_debug.pending, Some(KernelDebugOperation::DiscoverDefaults { .. })) {
                    "Finding selected build defaults… edits are preserved; review after discovery."
                } else {
                    app.kernel_debug.defaults_note.as_deref().unwrap_or("Attach/breakpoints can PAUSE execution; tracing adds overhead/sensitive data.")
                },
            );
            frame.render_widget(
                Paragraph::new(format!("{}\nReference: {}", tool.guide(), tool.reference()))
                    .wrap(Wrap { trim: false })
                    .scroll((dialog.guide_scroll, 0)),
                rows[2],
            );
            frame.render_widget(Paragraph::new(status).wrap(Wrap { trim: false }), rows[3]);
            frame.render_widget(Paragraph::new("Tab/↑/↓ field · ←/→/Space scope/boot mode · Ctrl+V paste · Ctrl+U clear\nEnter review exact launch · Esc cancel · PgUp/Dn guide").wrap(Wrap { trim: false }), rows[4]);
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
    if let Some(spec) = &app.kernel_debug.qemu_preview {
        lines.push(Line::from(format!("Boot mode: {}", spec.boot_mode.label())));
        lines.push(Line::from(format!(
            "Selected rootfs source: {}",
            spec.rootfs.display()
        )));
        lines.push(Line::from(
            "Runtime socket/log path: PRIVATE_SESSION is a placeholder allocated ONLY on launch.",
        ));
        if spec.boot_mode == yoctui_model::QemuDebugBootMode::OpenBmcRomulusFlash {
            lines.push(Line::from(format!(
                "Reference kernel (NOT a launch argument): {}",
                spec.kernel.display()
            )));
            lines.push(Line::from(
                "FLASH: private copy, snapshot/nonetwork, paused CPUs; no nokaslr injection.",
            ));
            lines.push(Line::from("Use hbreak start_kernel before firmware/MMU handoff; symbols must match flash kernel."));
            lines.push(Line::from("Quit stops owned guest; original flash stays unchanged. Relocation handling is manual."));
        } else {
            lines.push(Line::from(
                "QEMU: snapshot/nonetwork, paused CPUs (-S), nokaslr; quit stops owned guest.",
            ));
        }
        for (name, program, arguments) in [
            (
                "QEMU child",
                &spec.runqemu,
                spec.qemu_arguments(std::path::Path::new(
                    yoctui_model::QEMU_DEBUG_SOCKET_TEMPLATE,
                )),
            ),
            (
                "GDB child",
                &spec.gdb,
                spec.gdb_arguments(std::path::Path::new(
                    yoctui_model::QEMU_DEBUG_SOCKET_TEMPLATE,
                )),
            ),
        ] {
            lines.push(Line::from(format!("{name}: {}", program.display())));
            lines.extend(
                arguments
                    .iter()
                    .enumerate()
                    .map(|(index, arg)| Line::from(format!("argv[{index}]: {arg:?}"))),
            );
        }
    } else if let Some(preview) = &app.kernel_debug.serial_preview {
        lines.push(Line::from(
            "PHYSICAL BOARD: host file checks do NOT prove target readiness/build match.",
        ));
        lines.push(Line::from(
            "No automatic halt/reset/resume or SysRq. Ctrl+C cannot reliably halt kgdboc.",
        ));
        lines.push(Line::from(
            "After continue, re-entry may need manual SysRq-G; use deliberate detach.",
        ));
        lines.push(Line::from(format!(
            "Config inspected: {}",
            preview.spec.config.display()
        )));
        for name in yoctui_model::KGDB_CONFIG_OPTIONS {
            let value = preview
                .report
                .options
                .get(name)
                .and_then(|value| value.as_deref())
                .unwrap_or("absent/unknown");
            lines.push(Line::from(format!("{name}={value}")));
        }
        lines.push(Line::from("KDB optional; frame pointers aid backtraces; strict RWX may need hardware breakpoints."));
        lines.push(Line::from(preview.spec.boot_guidance()));
        lines.push(Line::from(format!(
            "GDB child: {}",
            preview.spec.gdb.display()
        )));
        lines.extend(
            preview
                .spec
                .gdb_arguments()
                .iter()
                .enumerate()
                .map(|(index, argument)| Line::from(format!("argv[{index}]: {argument:?}"))),
        );
    } else {
        lines.extend(
            dialog
                .request
                .arguments
                .iter()
                .enumerate()
                .map(|(index, argument)| Line::from(format!("argv[{index}]: {argument:?}"))),
        );
    }
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
