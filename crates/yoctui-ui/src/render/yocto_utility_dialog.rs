fn render_yocto_utility_dialog(frame: &mut Frame, app: &App, area: Rect) -> bool {
    let Some(Dialog::YoctoUtility(dialog)) = app.active_dialog() else {
        return false;
    };
    let fields = dialog.fields();
    let content_height = fields.len() as u16
        + 8
        + if dialog.validation_error.is_some() { 2 } else { 0 };
    let width = area.width.saturating_sub(8).clamp(52, 96);
    let height = content_height.min(area.height.saturating_sub(4)).max(10);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let mut lines = vec![
        Line::from(format!("Tool: {}", dialog.command.tool())),
        Line::from(format!("Workflow: {}", dialog.command.label())),
        Line::from(""),
    ];
    if fields.is_empty() {
        lines.push(Line::from("This command has no arguments."));
    } else {
        for (index, (label, value, kind)) in fields.iter().enumerate() {
            let value = if value.is_empty() {
                if label.contains("optional") {
                    "<not set>"
                } else {
                    "<required>"
                }
            } else {
                value
            };
            let suffix = if *kind == yoctui_model::YoctoUtilityFieldKind::Choice {
                "  ←/→"
            } else {
                ""
            };
            lines.push(Line::styled(
                format!(
                    "{} {:<24} {}{}",
                    if index == dialog.selected_field {
                        "▶"
                    } else {
                        " "
                    },
                    label,
                    value,
                    suffix
                ),
                selected_style(app, index == dialog.selected_field),
            ));
        }
    }
    if let Some(error) = &dialog.validation_error {
        lines.push(Line::from(""));
        lines.push(Line::styled(
            format!("Error: {error}"),
            ThemePalette::for_app(app).role(ThemePalette::for_app(app).error, Modifier::BOLD),
        ));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(
        "↑/↓ or Tab fields · ←/→ choices · type edits · Ctrl+U clears",
    ));
    lines.push(Line::from(
        "Enter reviews exact command · Esc cancels without spawning",
    ));
    frame.render_widget(
        Paragraph::new(lines)
            .block(dialog_block(app, dialog.command.label(), DialogTone::Standard))
            .wrap(Wrap { trim: false }),
        popup,
    );
    true
}
