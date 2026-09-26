fn render_dtc_decompile_dialog(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let Some(Dialog::DtcDecompile(dialog)) = app.active_dialog() else {
        return false;
    };
    let popup = dialog_popup_rect(area, 100, 24);
    clear_popup(frame, app, popup);
    let palette = ThemePalette::for_app(app);
    let mut lines = vec![Line::from(format!("Source: {}", dialog.source.display()))];
    let hint;
    if let Some(browser) = &dialog.browser {
        lines.push(Line::from("Choose the folder for the generated DTS:"));
        if let Some(directory) = &browser.directory {
            lines.push(Line::styled(
                directory.path.display().to_string(),
                palette.accent,
            ));
            let capacity = usize::from(popup.height.saturating_sub(9)).max(1);
            let start = browser.selection.saturating_sub(capacity.saturating_sub(1));
            for (index, path) in directory
                .children
                .iter()
                .enumerate()
                .skip(start)
                .take(capacity)
            {
                let label = path.file_name().unwrap_or_default().to_string_lossy();
                lines.push(Line::styled(
                    format!(
                        "{} {label}/",
                        if index == browser.selection { "▶" } else { " " }
                    ),
                    selected_style(app, index == browser.selection),
                ));
            }
        }
        if browser.loading {
            lines.push(Line::styled(
                format!(
                    "{} Loading folders…",
                    startup_activity_symbol(app.animation_frame as usize)
                ),
                palette.accent,
            ));
        }
        hint = "↑/↓ select · Enter/→ open · ←/Backspace parent · s use folder · Esc back";
    } else if let Some(editor) = &dialog.editor {
        lines.extend([
            Line::from("Edit absolute DTS destination:"),
            Line::from(""),
        ]);
        let before = &editor.text[..editor.cursor];
        let available = usize::from(popup.width.saturating_sub(8)).max(1);
        let suffix: String = before
            .chars()
            .rev()
            .take(available / 2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let after: String = editor.text[editor.cursor..]
            .chars()
            .take(available / 2)
            .collect();
        lines.push(Line::from(vec![
            Span::raw(suffix),
            Span::styled("|", Style::default().add_modifier(Modifier::REVERSED)),
            Span::raw(after),
        ]));
        hint = "Type/paste path · Left/Right Home/End · Ctrl-U clear · Enter accept · Esc discard";
    } else {
        let destination_selected = dialog.field == yoctui_model::DtcDecompileField::Destination;
        let view_selected = dialog.field == yoctui_model::DtcDecompileField::ViewAfter;
        lines.extend([
            Line::from(""),
            Line::styled(
                format!(
                    "{} Save as: {}",
                    if destination_selected { "▶" } else { " " },
                    dialog.output
                ),
                selected_style(app, destination_selected),
            ),
            Line::styled(
                format!(
                    "{} [{}] View file after decompilation",
                    if view_selected { "▶" } else { " " },
                    if dialog.view_after { "x" } else { " " }
                ),
                selected_style(app, view_selected),
            ),
            Line::from(""),
            Line::from("The generated DTS opens only after dtc exits successfully."),
        ]);
        hint = "Tab/↑/↓ field · b browse folders · e edit path · Space toggle · Enter review · Esc cancel";
    }
    if let Some(error) = &dialog.error {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            error.clone(),
            severity_style(app, yoctui_model::Severity::Error),
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(hint));
    frame.render_widget(
        Paragraph::new(lines)
            .style(palette.base())
            .block(dialog_block(app, "Decompile device tree", DialogTone::Standard))
            .wrap(Wrap { trim: false }),
        popup,
    );
    true
}
