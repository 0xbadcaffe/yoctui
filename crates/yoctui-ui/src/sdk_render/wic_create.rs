pub(crate) fn wic_field_value(dialog: &WicCreateDialog, field: WicCreateField) -> String {
    match field {
        WicCreateField::Machine => dialog.draft.machine.clone(),
        WicCreateField::Image => dialog.draft.image.clone(),
        WicCreateField::Kickstart => dialog.draft.kickstart.name.clone(),
        WicCreateField::OutputDirectory => dialog.draft.output_directory.clone(),
        WicCreateField::GenerateBmap => if dialog.draft.generate_bmap {
            "yes"
        } else {
            "no"
        }
        .into(),
        WicCreateField::Compression => match dialog.draft.compression {
            WicCompression::None => "none",
            WicCompression::Gzip => "gzip",
            WicCompression::Bzip2 => "bzip2",
            WicCompression::Xz => "xz",
        }
        .into(),
    }
}

pub(crate) fn wic_partition_summary(kickstart: &WicKickstart) -> String {
    if kickstart.partitions.is_empty() {
        return "none reported".into();
    }
    kickstart
        .partitions
        .iter()
        .enumerate()
        .map(|(index, partition)| {
            format!(
                "{}: mount={} fs={} source={} size={} MiB align={} KiB",
                index + 1,
                partition.mount_point.as_deref().unwrap_or("unavailable"),
                partition.filesystem.as_deref().unwrap_or("unavailable"),
                partition.source_plugin.as_deref().unwrap_or("unavailable"),
                partition
                    .size_mib
                    .map_or_else(|| "dynamic".into(), |value| value.to_string()),
                partition
                    .alignment_kib
                    .map_or_else(|| "unavailable".into(), |value| value.to_string()),
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn wic_limitations(kickstart: &WicKickstart) -> String {
    if kickstart.limitations.is_empty() {
        "none".into()
    } else {
        kickstart.limitations.join("\n")
    }
}

pub(crate) fn wic_create_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &WicCreateDialog,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, 20);
    clear_popup(frame, app, popup);
    let shell = dialog_shell(app, "Create Wic", DialogTone::Standard);
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
        WicCreateField::Machine,
        WicCreateField::Image,
        WicCreateField::Kickstart,
        WicCreateField::OutputDirectory,
        WicCreateField::GenerateBmap,
        WicCreateField::Compression,
    ];
    let labels = [
        "Machine",
        "Image",
        "Kickstart",
        "Output directory",
        "Generate bmap",
        "Compression",
    ];
    let rows = fields.into_iter().zip(labels).map(|(field, label)| {
        let selected = dialog.selected_field == field;
        let marker = if field.is_read_only() {
            " [read-only]"
        } else if selected && dialog.editing {
            " [editing]"
        } else {
            ""
        };
        Row::new([
            format!("{} {label}{marker}", if selected { "▶" } else { " " }),
            wic_field_value(dialog, field),
        ])
        .style(selected_style(app, selected))
    });
    frame.render_widget(
        Table::new(rows, [Constraint::Length(27), Constraint::Min(1)]).header(
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

pub(crate) fn wic_create_confirmation(
    frame: &mut Frame,
    app: &App,
    preview: &WicCreatePreview,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 100, area.height.saturating_sub(2).min(30));
    clear_popup(frame, app, popup);
    let partitions = wic_partition_summary(&preview.kickstart);
    let limitations = wic_limitations(&preview.kickstart);
    let argv = preview
        .argv
        .iter()
        .enumerate()
        .map(|(index, argument)| format!("[{index}]={}", argument.display()))
        .collect::<Vec<_>>()
        .join("  ");
    let source_line_count = preview.kickstart.source.lines().count();
    let source_limit = if popup.height <= 22 { 2 } else { 6 };
    let mut lines = vec![
        Line::from("Confirm managed Wic creation"),
        Line::from(format!(
            "Machine: {} | Image: {}",
            preview.request.machine, preview.request.image
        )),
        Line::from(format!(
            "Kickstart: {} | Output: {}",
            preview.request.kickstart.name,
            preview.request.output_directory.display()
        )),
        Line::from(""),
        Line::from(format!(
            "Kickstart source (showing {} of {} lines):",
            source_line_count.min(source_limit),
            source_line_count
        )),
    ];
    let mut source = source_preview(
        &preview
            .kickstart
            .source
            .lines()
            .take(source_limit)
            .collect::<Vec<_>>()
            .join("\n"),
        preview
            .kickstart
            .identity
            .path
            .as_ref()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .unwrap_or("kickstart.wks"),
        app,
    );
    lines.append(&mut source.lines);
    lines.extend([
        Line::from(""),
        Line::from(format!("Partitions: {partitions}")),
        Line::from(format!("Limitations: {limitations}")),
        Line::from(""),
        Line::from(format!("Exact argument vector: {argv}")),
        Line::from(""),
        Line::from("Enter confirms creation. Esc closes."),
    ]);
    frame.render_widget(
        Paragraph::new(Text::from(lines))
            .block(dialog_block(
                app,
                "Confirm managed Wic creation",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
        popup,
    );
}
