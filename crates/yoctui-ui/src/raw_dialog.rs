//! Raw dialog.
use super::*;

pub(crate) fn indexed_arguments(arguments: &[String]) -> String {
    arguments
        .iter()
        .enumerate()
        .map(|(index, argument)| format!("[{index}] {argument}"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn raw_form_selector_text(
    app: &App,
    command: &yoctui_model::RawCommand,
    parameter: &yoctui_model::RawParameter,
) -> String {
    match yoctui_app::raw_form_parameter_selector(app, command, &parameter.id) {
        Ok(selector) => match selector.inventory {
            yoctui_model::RawSelectorInventory::Available { choices } => format!(
                "Selector: {} choice{} (authoritative){}",
                choices.len(),
                if choices.len() == 1 { "" } else { "s" },
                if selector.manual_entry {
                    " · manual allowed"
                } else {
                    ""
                }
            ),
            yoctui_model::RawSelectorInventory::Unavailable { reason } => format!(
                "Selector unavailable: {reason}{}",
                if selector.manual_entry {
                    " · manual entry allowed"
                } else {
                    ""
                }
            ),
        },
        Err(yoctui_model::RawSelectorError::NotInventoryBacked { .. }) => {
            "Selector: manual entry".into()
        }
        Err(error) => format!("Selector unavailable: {error}"),
    }
}

pub(crate) fn raw_command_form_dialog(
    frame: &mut Frame,
    app: &App,
    form: &yoctui_model::RawCommandForm,
    area: Rect,
) {
    let popup = dialog_popup_rect(area, 110, 30);
    clear_popup(frame, app, popup);
    let block = dialog_block(app, "Run BitBake Command", DialogTone::Standard);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.is_empty() {
        return;
    }
    let catalog = yoctui_model::builtin_raw_catalog();
    let Some(command) = catalog.command(&form.command) else {
        frame.render_widget(
            Paragraph::new("The Raw form command is stale.\n\nq/Esc closes without execution.")
                .wrap(Wrap { trim: false }),
            inner,
        );
        return;
    };
    let sections = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(4),
        Constraint::Length(3),
    ])
    .split(inner);
    frame.render_widget(
        Paragraph::new(format!(
            "Template: {}\nCatalog: {} · Capability generation: {}\nBuild directory: {}",
            raw_command_template(command),
            catalog.version,
            form.capability_generation,
            form.build_directory.display()
        ))
        .wrap(Wrap { trim: false }),
        sections[0],
    );

    let total_fields = form.field_order.len().saturating_add(1);
    let field_rows = 2usize;
    let visible = usize::from(sections[1].height) / field_rows;
    let viewport = yoctui_model::centered_viewport_range(
        Some(form.field_selection.min(total_fields.saturating_sub(1))),
        total_fields,
        visible.max(1),
    );
    let position =
        BoundedScrollIndicator::new(viewport.start, viewport.len(), total_fields).label();
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    for index in viewport {
        let selected = index == form.field_selection;
        let marker = if selected { "▶" } else { " " };
        if let Some(parameter_id) = form.field_order.get(index) {
            let Some(parameter) = command
                .parameters
                .iter()
                .find(|parameter| &parameter.id == parameter_id)
            else {
                lines.push(Line::from(format!(
                    "{marker} Stale parameter: {parameter_id}"
                )));
                lines.push(Line::from("  The catalog definition is unavailable."));
                continue;
            };
            let Some(field) = form.fields.get(parameter_id) else {
                lines.push(Line::from(format!("{marker} {}", parameter.label)));
                lines.push(Line::from("  Form state unavailable."));
                continue;
            };
            let value = if field.editor.text.is_empty() {
                match parameter.presence {
                    yoctui_model::RawParameterPresence::Required => "<required>",
                    yoctui_model::RawParameterPresence::Optional => "<optional>",
                }
            } else {
                &field.editor.text
            };
            let mode = if field.editor.editing {
                "INSERT"
            } else {
                "NORMAL"
            };
            lines.push(Line::styled(
                bounded_cell_text(
                    &format!(
                        "{marker} {} {} · {} · {} · {mode}",
                        parameter.label,
                        parameter.placeholder,
                        raw_parameter_kind_label(parameter.kind),
                        raw_parameter_presence_label(parameter.presence)
                    ),
                    sections[1].width,
                ),
                if selected {
                    palette.selected()
                } else {
                    palette.base()
                },
            ));
            let detail = field.validation_error.as_ref().map_or_else(
                || {
                    format!(
                        "  Value: {value} · {}",
                        raw_form_selector_text(app, command, parameter)
                    )
                },
                |error| format!("  ERROR: {error} · Value: {value}"),
            );
            lines.push(Line::styled(
                bounded_cell_text(&detail, sections[1].width),
                field.validation_error.as_ref().map_or_else(
                    || palette.base(),
                    |_| palette.role(palette.error, Modifier::BOLD),
                ),
            ));
        } else {
            let editor = &form.additional_arguments;
            let value = if editor.editor.text.is_empty() {
                "<none>"
            } else {
                &editor.editor.text
            };
            let mode = if editor.editor.editing {
                "INSERT"
            } else {
                "NORMAL"
            };
            lines.push(Line::styled(
                format!("{marker} Additional arguments · {mode}"),
                if selected {
                    palette.selected()
                } else {
                    palette.base()
                },
            ));
            let detail = editor.validation_error.as_ref().map_or_else(
                || format!("  Value: {value} · native argv only; no shell evaluation"),
                |error| format!("  ERROR: {error} · Value: {value}"),
            );
            lines.push(Line::styled(
                bounded_cell_text(&detail, sections[1].width),
                editor.validation_error.as_ref().map_or_else(
                    || palette.base(),
                    |_| palette.role(palette.error, Modifier::BOLD),
                ),
            ));
        }
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().title(format!("Fields · {position}"))),
        sections[1],
    );
    let editing = form
        .field_order
        .get(form.field_selection)
        .and_then(|parameter| form.fields.get(parameter))
        .map_or(form.additional_arguments.editor.editing, |field| {
            field.editor.editing
        });
    let hints = if editing {
        "INSERT · type/backspace edit · ←/→ cursor · Esc Normal\nEnter validates and opens exact preview · Tab/Shift+Tab field"
    } else {
        "NORMAL · i edit · e replace · ←/→ authoritative selector · Tab/↑/↓ field\nEnter validates and opens exact preview · q/Esc close without execution"
    };
    frame.render_widget(
        Paragraph::new(hints).wrap(Wrap { trim: false }),
        sections[2],
    );
}

pub fn render_raw_execution_preview(
    frame: &mut Frame,
    preview: &yoctui_model::RawExecutionPreview,
    area: Rect,
) {
    if area.is_empty() {
        return;
    }
    let block = Block::default()
        .title("Run BitBake Command")
        .borders(Borders::ALL);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    let implementations = if preview.implementations.is_empty() {
        "none".into()
    } else {
        preview
            .implementations
            .iter()
            .map(|(capability, implementation)| format!("{}={implementation}", capability.as_str()))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut lines = vec![
        Line::from(format!("Command: {}", preview.command)),
        Line::from(format!(
            "Catalog: {}  Capability generation: {}",
            preview.catalog_version, preview.capability_generation
        )),
        Line::from(format!(
            "Build directory: {}",
            preview.build_directory.display()
        )),
        Line::from(format!(
            "Interaction: {:?}  Safety: {:?}",
            preview.interaction, preview.safety
        )),
        Line::from(format!("Implementations: {implementations}")),
    ];
    if preview.capability_issues.is_empty() {
        lines.push(Line::from("Capability limitations: none"));
    } else {
        lines.push(Line::from("Capability limitations:"));
        for issue in &preview.capability_issues {
            lines.push(Line::from(format!("- {}", issue.reason)));
            lines.extend(
                issue
                    .limitations
                    .iter()
                    .map(|limitation| Line::from(format!("  - {limitation}"))),
            );
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from("Exact indexed native argv:"));
    lines.extend(preview.indexed_arguments.iter().map(|argument| {
        let source = match argument.source {
            yoctui_model::RawPreviewArgumentSource::Executable => "executable".into(),
            yoctui_model::RawPreviewArgumentSource::Template { index } => {
                format!("template {index}")
            }
            yoctui_model::RawPreviewArgumentSource::Additional { index } => {
                format!("additional {index}")
            }
        };
        let value = if argument.value.is_empty() {
            "EMPTY".into()
        } else {
            argument.value.clone()
        };
        Line::from(format!("[{}] {value}  ({source})", argument.index))
    }));
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}
