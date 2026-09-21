pub(crate) fn test_result_import_dialog(
    frame: &mut Frame,
    app: &App,
    dialog: &yoctui_model::TestResultImportDialog,
    area: Rect,
) {
    let validation = dialog.validation_error.as_ref().map_or_else(
        || "✓ Validation: only the exact selected root is scanned within bounded limits.".into(),
        |error| format!("✕ Validation: {error}"),
    );
    testing_popup(
        frame,
        app,
        area,
        "Import structured test results",
        DialogTone::Standard,
        format!(
            "Normalized absolute testresults.json file or retained directory:\n{}_\n\n{}\nEnter imports; Esc cancels.",
            dialog.input, validation,
        ),
        10,
    );
}

pub(crate) fn toml_popup_editor(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    editor: &yoctui_model::PopupEditor,
    validation_error: Option<&str>,
) {
    let popup = dialog_popup_rect(area, 110, area.height.saturating_sub(4).min(34));
    clear_popup(frame, app, popup);
    let mode = textarea_mode_label(editor.mode());
    let content = popup_editor_text(editor);
    let block = dialog_block(app, format!("{title} — {mode}"), DialogTone::Standard);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    let diagnostic = textarea_diagnostic(editor, validation_error);
    let rows = Layout::vertical(if diagnostic.is_some() {
        vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(3),
        ]
    } else {
        vec![
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(3),
        ]
    })
    .split(inner);
    frame.render_widget(
        Paragraph::new(textarea_status(editor)).style(dialog_styles(app).hint),
        rows[0],
    );
    frame.render_widget(Paragraph::new(content).wrap(Wrap { trim: false }), rows[1]);
    let shortcuts = if let Some(diagnostic) = diagnostic {
        frame.render_widget(
            Paragraph::new(
                dialog_shell(app, title, DialogTone::Standard)
                    .validation(Some(diagnostic.as_str())),
            )
            .wrap(Wrap { trim: false }),
            rows[2],
        );
        rows[3]
    } else {
        rows[2]
    };
    let shell = dialog_shell(app, title, DialogTone::Standard);
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            shell.controls(
                Some(("Enter", "Save/preview")),
                &[("Esc", "Normal"), ("q", "Close")],
            ),
            Line::styled(
                "i insert  v visual  e change value  u undo  r redo  h/j/k/l move",
                dialog_styles(app).hint,
            ),
            Line::styled(
                "Ctrl+C copy  Ctrl+V paste  Home/End line  b/w word  PgUp/PgDn page",
                dialog_styles(app).hint,
            ),
        ]))
        .wrap(Wrap { trim: false }),
        shortcuts,
    );
}

pub(crate) fn popup_editor_text(editor: &yoctui_model::PopupEditor) -> String {
    let mut raw = String::with_capacity(editor.text.len() + 7);
    let selection = editor.selection;
    for (index, character) in editor.text.char_indices() {
        let empty_selection = selection.is_some_and(|(start, end)| start == index && end == index);
        if selection.is_some_and(|(start, _)| start == index) {
            raw.push('⟦');
        }
        if editor.cursor == index {
            raw.push('▏');
        }
        if empty_selection {
            raw.push('⟧');
        }
        raw.push(character);
        let next = index + character.len_utf8();
        if selection.is_some_and(|(start, end)| start < end && end == next) {
            raw.push('⟧');
        }
    }
    let empty_selection_at_end = selection
        .is_some_and(|(start, end)| start == editor.text.len() && end == editor.text.len());
    if selection.is_some_and(|(start, _)| start == editor.text.len()) {
        raw.push('⟦');
    }
    if editor.cursor == editor.text.len() {
        raw.push('▏');
    }
    if empty_selection_at_end
        || selection.is_some_and(|(start, end)| start < end && end == editor.text.len())
    {
        raw.push('⟧');
    }
    let line_count = editor.line_count();
    let number_width = line_count.to_string().len();
    raw.split('\n')
        .enumerate()
        .map(|(line, text)| {
            let diagnostic = editor.validation().iter().any(|span| {
                editor.text[..span.start]
                    .bytes()
                    .filter(|byte| *byte == b'\n')
                    .count()
                    == line
            });
            format!(
                "{:>number_width$} │ {}{}",
                line + 1,
                text,
                if diagnostic { "  [validation]" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn textarea_mode_label(mode: yoctui_model::TextAreaMode) -> &'static str {
    match mode {
        yoctui_model::TextAreaMode::Normal => "NORMAL",
        yoctui_model::TextAreaMode::Insert => "INSERT",
        yoctui_model::TextAreaMode::Visual => "VISUAL",
    }
}

pub fn checkbox_text(row: &yoctui_model::CheckboxState, unicode: bool) -> String {
    let focus = if row.focused { ">" } else { " " };
    let reason = row
        .disabled_reason
        .as_deref()
        .map_or(String::new(), |reason| format!(" — {reason}"));
    format!(
        "{focus} {} {} ({}){reason}",
        row.marker(unicode),
        row.label,
        row.semantic_state()
    )
}

pub(crate) fn textarea_status(editor: &yoctui_model::PopupEditor) -> String {
    let position = editor.position();
    let save = match editor.save_state() {
        yoctui_model::TextAreaSaveState::Clean { .. } => "clean",
        yoctui_model::TextAreaSaveState::Modified { .. } => "modified",
        yoctui_model::TextAreaSaveState::Preview { .. } => "diff preview",
        yoctui_model::TextAreaSaveState::Conflict { .. } => "external conflict",
        yoctui_model::TextAreaSaveState::Saving { .. } => "saving atomically",
        yoctui_model::TextAreaSaveState::Saved { .. } => "saved",
        yoctui_model::TextAreaSaveState::Failed {
            recoverable: true, ..
        } => "save failed · retry available",
        yoctui_model::TextAreaSaveState::Failed { .. } => "save failed",
    };
    let wrap = editor
        .layout()
        .wrap_width
        .map_or_else(|| "wrap off".to_owned(), |width| format!("wrap {width}"));
    let search = if editor.search_state().query.is_empty() {
        String::new()
    } else {
        format!(
            " · find {}/{}",
            editor.search_state().selected.map_or(0, |index| index + 1),
            editor.search_state().matches.len()
        )
    };
    format!(
        "{} · Ln {}, Col {} · UTF-8 · {} · {} lines · {}{}",
        textarea_mode_label(editor.mode()),
        position.line + 1,
        position.column + 1,
        save,
        editor.line_count(),
        wrap,
        search
    )
}

pub(crate) fn textarea_diagnostic(
    editor: &yoctui_model::PopupEditor,
    legacy: Option<&str>,
) -> Option<String> {
    if let Some(message) = legacy {
        return Some(format!("ERROR: {message}"));
    }
    if let Some(span) = editor.validation().first() {
        return Some(format!(
            "{:?} bytes {}..{}: {}",
            span.severity, span.start, span.end, span.message
        ));
    }
    match editor.save_state() {
        yoctui_model::TextAreaSaveState::Conflict { .. } => {
            Some("CONFLICT: file changed externally; review before saving".into())
        }
        yoctui_model::TextAreaSaveState::Failed { message, .. } => {
            Some(format!("SAVE FAILED: {message}"))
        }
        _ => None,
    }
}
