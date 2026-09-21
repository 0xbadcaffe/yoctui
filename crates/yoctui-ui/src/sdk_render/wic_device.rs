pub(crate) fn wic_device_lines(
    app: &App,
    devices: &[WicDevice],
    available_lines: usize,
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let selection = devices
        .iter()
        .position(|device| app.wic_device_selection.as_ref() == Some(&device.identity));
    let capacity = (available_lines / 3).max(1);
    let viewport = yoctui_model::centered_viewport_range(selection, devices.len(), capacity);
    for device in &devices[viewport] {
        let selected = app.wic_device_selection.as_ref() == Some(&device.identity);
        let style = selected_style(app, selected);
        let mounts = if device.descendant_mounts.is_empty() {
            "none".into()
        } else {
            device
                .descendant_mounts
                .iter()
                .map(|mount| mount.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        };
        lines.push(
            Line::from(format!(
                "{} {} | {} | {} | major:minor {}",
                if selected { "▶" } else { " " },
                device.identity.path.display(),
                format_bytes(device.identity.size_bytes),
                device.identity.size_bytes,
                device.identity.major_minor,
            ))
            .style(style),
        );
        lines.push(
            Line::from(format!(
                "  model={} | serial={} | transport={} | removable={} | writable={} | read-only={} | mounts={}",
                device.identity.model.as_deref().unwrap_or("unavailable"),
                device.identity.serial.as_deref().unwrap_or("unavailable"),
                device.identity.transport.as_deref().unwrap_or("unavailable"),
                device.removable,
                device.writable,
                device.read_only,
                mounts,
            ))
            .style(style),
        );
        if let Some(reason) = &device.unavailable_reason {
            lines.push(Line::from(format!("  unavailable: {reason}")).style(style));
        }
    }
    lines
}

pub(crate) fn wic_device_picker(
    frame: &mut Frame,
    app: &App,
    dialog: &WicDevicePickerDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(2).min(30));
    let device_lines = usize::from(popup.height.saturating_sub(9)).max(1);
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
            dialog.request.image.modified_unix_seconds,
        )),
        Line::from(
            "Only removable, writable whole devices without mounted descendants are eligible.",
        ),
        Line::from(""),
    ];
    match &app.wic_devices {
        WicDeviceInventoryState::Loading { request } if request == &dialog.request => {
            lines.push(Line::from("Discovering removable whole devices…"));
        }
        WicDeviceInventoryState::Available { request, devices } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices, device_lines));
            }
            lines.push(Line::from(""));
            lines.push(Line::from("Discovery limitations: none"));
        }
        WicDeviceInventoryState::Partial {
            request,
            devices,
            limitations,
        } if request == &dialog.request => {
            if devices.is_empty() {
                lines.push(Line::from(
                    "No eligible removable whole devices were found.",
                ));
            } else {
                lines.extend(wic_device_lines(app, devices, device_lines));
            }
            lines.push(Line::from(""));
            lines.push(Line::from(format!(
                "Discovery limitations: {}",
                limitations.join("; ")
            )));
        }
        WicDeviceInventoryState::Failed { request, message } if request == &dialog.request => {
            lines.push(Line::from(format!("Device discovery failed: {message}")));
        }
        _ => lines.push(Line::from(
            "This device inventory is stale; close and start a new discovery.",
        )),
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "↑/↓ selects. Enter opens the required phrase dialog. Esc closes.",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Select protected Wic write device",
                DialogTone::Standard,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_write_phrase_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &WicWritePhraseDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 15);
    clear_popup(frame, app, popup);
    let expected = format!("WRITE {}", dialog.device.path.display());
    let mut lines = vec![
        Line::from(format!(
            "Image: {} | {} bytes",
            dialog.request.image.path.display(),
            dialog.request.image.size_bytes,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            dialog.device.path.display(),
            dialog.device.major_minor,
            dialog.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            dialog.device.model.as_deref().unwrap_or("unavailable"),
            dialog.device.serial.as_deref().unwrap_or("unavailable"),
            dialog.device.transport.as_deref().unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from(format!("Required phrase: {expected}")),
        Line::from(format!("Input: {}_", dialog.input)),
    ];
    if let Some(error) = &dialog.validation_error {
        let palette = ThemePalette::for_app(app);
        lines.push(
            Line::from(format!("✕ Validation: {error}"))
                .style(palette.role(palette.error, Modifier::BOLD)),
        );
    }
    lines.extend([
        Line::from(""),
        Line::from(
            "The phrase alone does not write. Enter opens a separate exact command preview.",
        ),
        Line::from("Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Confirm protected Wic device identity",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_write_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &WicWritePreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(2).min(25));
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from("DESTRUCTIVE OPERATION: this overwrites the selected whole device."),
        Line::from(""),
        Line::from(format!(
            "Image: {} | {} bytes | modified {}s",
            preview.request.image.path.display(),
            preview.request.image.size_bytes,
            preview.request.image.modified_unix_seconds,
        )),
        Line::from(format!(
            "Device: {} | major:minor {} | {} bytes",
            preview.request.device.path.display(),
            preview.request.device.major_minor,
            preview.request.device.size_bytes,
        )),
        Line::from(format!(
            "Model: {} | Serial: {} | Transport: {}",
            preview
                .request
                .device
                .model
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .serial
                .as_deref()
                .unwrap_or("unavailable"),
            preview
                .request
                .device
                .transport
                .as_deref()
                .unwrap_or("unavailable"),
        )),
        Line::from(""),
        Line::from("Exact argument vector:"),
    ];
    lines.extend(
        preview
            .argv
            .iter()
            .enumerate()
            .map(|(index, argument)| Line::from(format!("[{index}]={}", argument.display()))),
    );
    lines.extend([
        Line::from(""),
        Line::from("Enter starts WRITE DEVICE. Esc closes without writing."),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(
                app,
                "Final protected Wic device-write preview",
                DialogTone::Destructive,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn wic_cancellation_confirmation(
    frame: &mut Frame,
    app: &App,
    id: WicSessionId,
    incomplete_device_warning: bool,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 84, 10);
    clear_popup(frame, app, popup);
    let detail = app.wic_session(id).map_or_else(
        || format!("Wic operation {} is unavailable.", id.0),
        |session| match &session.operation {
            WicOperation::Create(request) => format!(
                "Cancel Wic creation {}?\nImage: {}\nOutput: {}",
                id.0,
                request.image,
                request.output_directory.display()
            ),
            WicOperation::Write(request) => format!(
                "Cancel Wic device write {}?\nImage: {}\nDevice: {}",
                id.0,
                request.image.path.display(),
                request.device.path.display()
            ),
        },
    );
    let warning = if incomplete_device_warning {
        "\nWARNING: stopping a device write can leave the target incomplete and unusable."
    } else {
        ""
    };
    let title = if incomplete_device_warning {
        "Confirm Wic device-write cancellation"
    } else {
        "Confirm Wic cancellation"
    };
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}{warning}\n\nEnter confirms cancellation. Esc keeps it running."
        ))
        .block(dialog_block(
            app,
            title,
            if incomplete_device_warning {
                DialogTone::Destructive
            } else {
                DialogTone::Confirmation
            },
        ))
        .wrap(Wrap { trim: false }),
        popup,
    );
}
