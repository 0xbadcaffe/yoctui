//! Editor render.
use super::*;

#[allow(dead_code)]
pub(crate) fn build_progress_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(14).clamp(50, 110);
    let height = area.height.saturating_sub(6).clamp(12, 28);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    let mut task_lines = app
        .tasks
        .values()
        .map(|task| {
            format!(
                "  {:<28} {:<18} {:>3}%",
                task.recipe,
                task.task,
                task.progress.unwrap_or(0)
            )
        })
        .collect::<Vec<_>>();
    task_lines.sort();
    if task_lines.is_empty() {
        task_lines.push("  Waiting for BitBake task events…".into());
    }
    let cpu = app
        .host_telemetry
        .cpu_utilization_percent
        .map_or_else(|| "sampling".into(), |value| format!("{value}%"));
    let disk = app
        .host_telemetry
        .disk_available_bytes
        .map_or_else(|| "unavailable".into(), format_bytes);
    let parse = match (app.build.parse_current, app.build.parse_total) {
        (Some(current), Some(total)) if total > 0 => format!(
            "{current}/{total} ({:.0}%)",
            current as f64 / total as f64 * 100.0
        ),
        _ => "not parsing".into(),
    };
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Target: {}\nStatus: {:?}    Tasks: {} complete, {} active\nParse: {parse}    CPU: {cpu}    Free disk: {disk}\n\nActive recipe tasks:\n{}\n\nBitBake is running. c cancels the build.",
            app.build.target.as_deref().unwrap_or("unknown"),
            app.build.status,
            app.build.completed,
            app.tasks.len(),
            task_lines.join("\n"),
        ))
        .block(Block::default().title("Build progress").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        popup,
    );
}

pub(crate) fn build_completion_popup(frame: &mut Frame, app: &App, area: Rect) {
    let width = area.width.saturating_sub(24).clamp(44, 90);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        area.height.saturating_sub(9) / 2,
        width,
        9,
    );
    let result = match app.build.status {
        yoctui_model::BuildStatus::Completed => "completed successfully",
        yoctui_model::BuildStatus::Cancelled => "was cancelled",
        _ => "failed",
    };
    let action = if app.build.status == yoctui_model::BuildStatus::Failed && app.build.errors > 0 {
        "Press Enter to investigate Errors; any other key returns to Yoctui."
    } else {
        "Press any key to return to Yoctui."
    };
    let elapsed = app.build_summary_at(SystemTime::now()).elapsed;
    clear_popup(frame, app, popup);
    frame.render_widget(
        Paragraph::new(format!(
            "Build {} for {}.\n\nTasks completed: {}\nWarnings: {}    Errors: {}    Exit code: {}\nElapsed: {}\n\n{}",
            result,
            app.build.target.as_deref().unwrap_or("unknown target"),
            app.build.completed,
            app.build.warnings,
            app.build.errors,
            app.build.exit_code.map_or_else(|| "unknown".into(), |code| code.to_string()),
            elapsed.map(format_duration).unwrap_or_else(|| "unknown".into()),
            action,
        ))
        .style(build_status_style(app))
        .block(dialog_block(app, "Build finished", DialogTone::Result))
        .wrap(Wrap { trim: true }),
        popup,
    );
}
pub(crate) fn recipe_editor(frame: &mut Frame, app: &App, editor: &RecipeEditor, area: Rect) {
    let integrated = area.width >= 150 && area.height >= 50;
    let [header, footer] = yoctui_app::workbench_chrome_heights(app, area.width, area.height);
    let navigator = yoctui_app::workbench_pane_widths(app, area.width, area.height)[0];
    let popup = if integrated {
        Rect::new(
            navigator,
            header,
            area.width.saturating_sub(navigator),
            area.height.saturating_sub(header + footer),
        )
    } else {
        let width = area.width.saturating_sub(4).max(30);
        let height = area.height.saturating_sub(2).max(8);
        Rect::new(
            (area.width.saturating_sub(width)) / 2,
            (area.height.saturating_sub(height)) / 2,
            width,
            height,
        )
    };
    clear_popup(frame, app, popup);
    let mut outer = dialog_block(
        app,
        format!("Recipe editor: {}", editor.recipe),
        DialogTone::Standard,
    );
    if app.menu.is_open() {
        outer = outer.border_style(ThemePalette::for_app(app).role(
            ThemePalette::for_app(app).inactive_border,
            Modifier::empty(),
        ));
    }
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);
    let regions = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(7),
        Constraint::Length(1),
    ])
    .split(inner);
    let columns = if integrated {
        Layout::horizontal([
            Constraint::Percentage(18),
            Constraint::Percentage(57),
            Constraint::Percentage(25),
        ])
        .split(inner)
    } else {
        Layout::horizontal([Constraint::Percentage(35), Constraint::Percentage(65)])
            .split(regions[0])
    };
    let editor_rows = Layout::vertical([
        Constraint::Min(4),
        Constraint::Length(7),
        Constraint::Length(1),
    ])
    .split(columns[1]);
    let document_area = if integrated {
        editor_rows[0]
    } else {
        columns[1]
    };
    let validation_area = if integrated {
        editor_rows[1]
    } else {
        regions[1]
    };
    let status_area = if integrated {
        editor_rows[2]
    } else {
        regions[2]
    };
    let files = editor
        .files
        .iter()
        .enumerate()
        .map(|(index, path)| {
            format!(
                "{} {}",
                if index == editor.selection {
                    "▶"
                } else {
                    " "
                },
                path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    frame.render_widget(
        Paragraph::new(files)
            .block(
                Block::default()
                    .title(format!("Workspace file tree: {}", editor.recipe))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        columns[0],
    );
    let selected = editor
        .files
        .get(editor.selection)
        .map_or_else(|| "no file".into(), |path| path.display().to_string());
    let mode = textarea_mode_label(editor.document.mode());
    let modified = if editor.is_dirty() {
        "modified"
    } else {
        "clean"
    };
    let position = editor.document.position();
    let content = popup_editor_text(&editor.document);
    let file_focus = editor.focus == yoctui_model::RecipeEditorFocus::Files;
    let document_focus = editor.focus == yoctui_model::RecipeEditorFocus::Document;
    frame.render_widget(
        Paragraph::new(source_preview(&content, &selected, app))
            .block(
                Block::default()
                    .title(format!(
                        "{} {selected} — {} · {mode} · {modified} · Ln {} Col {}",
                        if document_focus { "▶" } else { " " },
                        editor.language.label(),
                        position.line + 1,
                        position.column + 1,
                    ))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        document_area,
    );
    if integrated {
        let layer = app
            .workspace
            .recipes
            .iter()
            .find(|recipe| recipe.name == editor.recipe)
            .and_then(|recipe| recipe.layer.as_deref())
            .unwrap_or("unknown");
        let inspector = vec![
            Line::styled(
                "Recipe",
                ThemePalette::for_app(app)
                    .role(ThemePalette::for_app(app).informational, Modifier::BOLD),
            ),
            Line::from(format!("Name: {0}", editor.recipe)),
            Line::from(format!("Layer: {layer}")),
            Line::from(format!("File: {selected}")),
            Line::from(format!("Language: {}", editor.language.label())),
            Line::from(format!("State: {modified}")),
            Line::default(),
            Line::styled(
                "Actions",
                ThemePalette::for_app(app)
                    .role(ThemePalette::for_app(app).informational, Modifier::BOLD),
            ),
            Line::from("[v] Validate"),
            Line::from("[p] Preview diff"),
            Line::from("[Ctrl+S] Save"),
            Line::from("[Ctrl+B] Build recipe"),
            Line::from("[e] External editor"),
        ];
        frame.render_widget(
            Paragraph::new(inspector)
                .block(pane_block(app, "Recipe Inspector", false))
                .wrap(Wrap { trim: false }),
            columns[2],
        );
    }
    frame.render_widget(
        Paragraph::new({
            let diagnostics = editor.local_validation();
            let validation = if diagnostics.is_empty() {
                "Local validation: ✓ no structural diagnostics".into()
            } else {
                format!("Local validation: ✕ {}", diagnostics.join(" · "))
            };
            let diff = editor.diff_preview(2);
            let mut lines = vec![
                Line::from(validation),
                Line::from("Diff preview: loaded → buffer"),
            ];
            if diff.is_empty() {
                lines.push(Line::from("  unchanged"));
            } else {
                lines.extend(diff.into_iter().map(Line::from));
            }
            lines.push(Line::from(if editor.searching {
                format!(
                    "Search /{}▏ · Enter finish · n/N next/previous",
                    editor.document.search_state().query
                )
            } else {
                "Structural diagnostics only; BitBake/compiler output remains authoritative".into()
            }));
            lines
        })
        .block(
            Block::default()
                .title("Validation and diff state")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        validation_area,
    );
    frame.render_widget(
        Paragraph::new(if file_focus {
            "FILES · ↑/↓ select · Enter/e focus editor · Ctrl+B build recipe · Esc close"
        } else if integrated {
            "i insert · / search · Ctrl+S save · Ctrl+B build · Tab files"
        } else {
            "EDITOR · i insert · v visual · / search · Ctrl+S save · Ctrl+B build · Tab files"
        })
        .style(dialog_styles(app).hint),
        status_area,
    );
}
