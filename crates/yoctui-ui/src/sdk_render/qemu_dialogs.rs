pub(crate) fn qemu_launch_field_label(field: QemuLaunchField) -> &'static str {
    match field {
        QemuLaunchField::Machine => "Machine",
        QemuLaunchField::Image => "Image",
        QemuLaunchField::Kernel => "Kernel",
        QemuLaunchField::Rootfs => "Root filesystem",
        QemuLaunchField::Networking => "Networking",
        QemuLaunchField::Memory => "Memory MiB",
        QemuLaunchField::Display => "Display",
        QemuLaunchField::Serial => "Serial",
        QemuLaunchField::ExtraArguments => "Extra arguments",
    }
}

pub(crate) fn image_console_field_label(field: ImageConsoleField) -> &'static str {
    match field {
        ImageConsoleField::Mode => "Mode",
        ImageConsoleField::Image => "Selected image",
        ImageConsoleField::Networking => "QEMU networking",
        ImageConsoleField::Memory => "QEMU memory MiB",
        ImageConsoleField::Host => "SSH host",
        ImageConsoleField::User => "SSH user",
        ImageConsoleField::Port => "SSH port",
        ImageConsoleField::IdentityFile => "SSH identity file",
    }
}

pub(crate) fn image_console_field_value(
    dialog: &ImageConsoleDialog,
    field: ImageConsoleField,
) -> String {
    match field {
        ImageConsoleField::Mode => dialog.draft.mode.label().into(),
        ImageConsoleField::Image => dialog.draft.image.path.display().to_string(),
        ImageConsoleField::Networking => format!("{:?}", dialog.draft.networking),
        ImageConsoleField::Memory => dialog.draft.memory_mib.clone(),
        ImageConsoleField::Host => {
            if dialog.draft.host.is_empty() {
                "required".into()
            } else {
                dialog.draft.host.clone()
            }
        }
        ImageConsoleField::User => dialog.draft.user.clone(),
        ImageConsoleField::Port => dialog.draft.port.clone(),
        ImageConsoleField::IdentityFile => {
            if dialog.draft.identity_file.is_empty() {
                "default SSH configuration".into()
            } else {
                dialog.draft.identity_file.clone()
            }
        }
    }
}

pub(crate) fn image_console_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &ImageConsoleDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 104, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Image Console", DialogTone::Standard);
    let block = shell.clone().block();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let regions = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(5),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(inner);
    let purpose = match dialog.draft.mode {
        ImageConsoleMode::Qemu => format!(
            "Boots {} with runqemu in a daemon-owned PTY. nographic + serialstdio are enforced.",
            dialog.draft.image.image
        ),
        ImageConsoleMode::Ssh => format!(
            "Connects to an already-running target. OpenSSH host-key policy stays enabled. {}",
            app.ssh_client_capability.status_text()
        ),
    };
    frame.render_widget(
        Paragraph::new(purpose).wrap(Wrap { trim: true }),
        regions[0],
    );
    let rows = dialog.fields().iter().copied().map(|field| {
        let selected = field == dialog.selected_field;
        Row::new([
            format!(
                "{} {}{}",
                if selected { "▶" } else { " " },
                image_console_field_label(field),
                if field.is_read_only() {
                    " [read-only]"
                } else {
                    ""
                }
            ),
            image_console_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(26), Constraint::Min(1)])
            .header(Row::new(["Field", "Value"]).style(Style::default().bold())),
        regions[1],
    );
    let validation = dialog.validation_error.as_deref().map_or_else(
        || match dialog.draft.mode {
            ImageConsoleMode::Qemu => format!("runqemu: {}", qemu_capability_text(app)),
            ImageConsoleMode::Ssh => {
                "Password prompts stay inside the PTY; credentials are never stored.".into()
            }
        },
        |message| format!("Cannot launch: {message}"),
    );
    frame.render_widget(
        Paragraph::new(validation).wrap(Wrap { trim: true }),
        regions[2],
    );
    frame.render_widget(
        Paragraph::new(shell.controls(
            Some(("Enter", "Launch")),
            &[("↑/↓", "Field"), ("←/→", "Choice"), ("Esc", "Cancel")],
        )),
        regions[3],
    );
}

pub(crate) fn qemu_launch_field_value(dialog: &QemuLaunchDialog, field: QemuLaunchField) -> String {
    match field {
        QemuLaunchField::Machine => dialog.draft.machine.clone(),
        QemuLaunchField::Image => dialog.draft.image.path.display().to_string(),
        QemuLaunchField::Kernel => {
            if dialog.draft.kernel.is_empty() {
                "not set".into()
            } else {
                dialog.draft.kernel.clone()
            }
        }
        QemuLaunchField::Rootfs => {
            if dialog.draft.rootfs.is_empty() {
                "not set".into()
            } else {
                dialog.draft.rootfs.clone()
            }
        }
        QemuLaunchField::Networking => match dialog.draft.networking {
            QemuNetworkingMode::Slirp => "slirp",
            QemuNetworkingMode::Tap => "tap",
            QemuNetworkingMode::None => "none",
        }
        .into(),
        QemuLaunchField::Memory => dialog.draft.memory_mib.clone(),
        QemuLaunchField::Display => match dialog.draft.display {
            QemuDisplayMode::Graphical => "graphical",
            QemuDisplayMode::Nographic => "nographic",
        }
        .into(),
        QemuLaunchField::Serial => match dialog.draft.serial {
            QemuSerialMode::Stdio => "stdio",
            QemuSerialMode::Telnet => "telnet",
            QemuSerialMode::None => "none",
        }
        .into(),
        QemuLaunchField::ExtraArguments => {
            if dialog.draft.extra_arguments.is_empty() {
                "none".into()
            } else {
                dialog.draft.extra_arguments.clone()
            }
        }
    }
}

pub(crate) fn qemu_launch_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &QemuLaunchDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Launch runqemu", DialogTone::Standard);
    let block = shell.clone().block();
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let regions = Layout::vertical(if dialog.validation_error.is_some() {
        vec![
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ]
    } else {
        vec![Constraint::Min(4), Constraint::Length(1)]
    })
    .split(inner);
    let fields = [
        QemuLaunchField::Machine,
        QemuLaunchField::Image,
        QemuLaunchField::Kernel,
        QemuLaunchField::Rootfs,
        QemuLaunchField::Networking,
        QemuLaunchField::Memory,
        QemuLaunchField::Display,
        QemuLaunchField::Serial,
        QemuLaunchField::ExtraArguments,
    ];
    let rows = fields.into_iter().map(|field| {
        let selected = dialog.selected_field == field;
        let suffix = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!(
                "{} {}{}",
                if selected { "▶" } else { " " },
                qemu_launch_field_label(field),
                suffix
            ),
            qemu_launch_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(23), Constraint::Min(1)]).header(
            Row::new(["Field", "Value"]).style(
                ThemePalette::for_app(app)
                    .role(ThemePalette::for_app(app).table_header, Modifier::BOLD),
            ),
        ),
        regions[0],
    );
    if let Some(message) = &dialog.validation_error {
        frame.render_widget(
            Paragraph::new(shell.validation(Some(message))).wrap(Wrap { trim: false }),
            regions[1],
        );
    }
    let controls = *regions.last().expect("dialog has controls");
    frame.render_widget(
        Paragraph::new(shell.controls(
            Some(("p", "Preview")),
            &[
                ("↑/↓", "Field"),
                ("←/→", "Choice"),
                ("Enter", "Edit"),
                ("Esc", "Close"),
            ],
        )),
        controls,
    );
}

pub(crate) fn qemu_launch_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &QemuLaunchPreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 18);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!("Machine: {}", preview.request.machine)),
        Line::from(format!("Image: {}", preview.request.image.image)),
        Line::from(format!(
            "Artifact: {}",
            preview.request.image.path.display()
        )),
        Line::from("Exact argument vector (one argument per line):"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}] {}", argument.display()))),
    );
    lines.push(Line::from(""));
    lines.push(Line::from(
        "Enter confirms launch. Esc closes without launch.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Confirm managed runqemu launch",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn qemu_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: QemuSessionId,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 72, 7);
    clear_popup(frame, app, popup);
    let detail = app.qemu_session(id).map_or_else(
        || format!("Session {} is no longer available.", id.0),
        |session| {
            format!(
                "Cancel managed session {}?\nImage: {}\nArtifact: {}",
                id.0,
                session.request.image.image,
                session.request.image.path.display()
            )
        },
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(dialog_block(
            app,
            "Confirm runqemu cancellation",
            DialogTone::Confirmation,
        ))
        .wrap(Wrap { trim: false }),
        popup,
    );
}
