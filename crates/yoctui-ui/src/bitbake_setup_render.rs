//! The native review form surrounds, rather than reimplements, upstream setup.
use super::*;
use yoctui_model::BitBakeSetupWizard;

pub(crate) fn catalogue(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.bitbake_setup;
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Build environment · 2 Setups")
        .style(ThemePalette::for_app(app).base());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(inner);
    frame.render_widget(Paragraph::new(format!(
        "1 Environment | 2 Setups | w/n new setup | o choose top directory\nTop directory: {}\nEnter import selected profile · no build/daemon switch starts\n{}",
        state.top_dir, if state.loading {"Loading directories…"} else {"Locally discovered setups (not yet verified)"}
    )), rows[0]);
    let capacity = usize::from(rows[1].height).max(1);
    let start = state.selected.saturating_sub(capacity.saturating_sub(1));
    let lines = if state.entries.is_empty() {
        vec![Line::from(
            "No setups here. w creates one; o selects an existing top directory.",
        )]
    } else {
        state
            .entries
            .iter()
            .enumerate()
            .skip(start)
            .take(capacity)
            .map(|(index, entry)| {
                Line::styled(
                    format!(
                        "{} {} · {}",
                        if index == state.selected { ">" } else { " " },
                        entry.name,
                        entry.profile.build_dir.display()
                    ),
                    if index == state.selected {
                        selected_style(app, true)
                    } else {
                        ThemePalette::for_app(app).base()
                    },
                )
            })
            .collect()
    };
    frame.render_widget(Paragraph::new(lines), rows[1]);
    frame.render_widget(
        Paragraph::new(format!(
            "{}\nf/3 fragments · s status · u update · r refresh · Esc environment",
            state.error.as_deref().unwrap_or(
                "Creation may download sources; only trusted templates/registries should be used."
            )
        ))
        .wrap(Wrap { trim: false }),
        rows[2],
    );
}

pub(crate) fn wizard(frame: &mut Frame, app: &App, wizard: &BitBakeSetupWizard, area: Rect) {
    let width = area.width.saturating_sub(4).min(110);
    let height = area.height.saturating_sub(2).min(26);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let title = if wizard.root_only {
        "Choose setup top directory"
    } else if wizard.review {
        "Review new BitBake setup"
    } else {
        "New BitBake setup"
    };
    frame.render_widget(dialog_block(app, title, DialogTone::Standard), popup);
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(3),
        Constraint::Length(2),
    ])
    .split(popup.inner(Margin::new(1, 1)));
    let mut lines = Vec::new();
    let hint;
    if let Some(editor) = &wizard.editor {
        lines.push(Line::from(format!(
            "Edit {}",
            BitBakeSetupWizard::LABELS[wizard.field]
        )));
        lines.push(Line::from(
            "Selected text is replaced when typing or pasting.",
        ));
        lines.push(Line::from(""));
        let available = usize::from(rows[0].width.saturating_sub(2)) / 2;
        let before: String = editor.text[..editor.cursor]
            .chars()
            .rev()
            .take(available)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let after: String = editor.text[editor.cursor..]
            .chars()
            .take(available)
            .collect();
        let style = if editor.selection.is_some() {
            selected_style(app, true)
        } else {
            ThemePalette::for_app(app).base()
        };
        lines.push(Line::from(vec![
            Span::styled(before, style),
            Span::styled("|", Style::default().add_modifier(Modifier::REVERSED)),
            Span::styled(after, style),
        ]));
        hint = "Type/paste · Ctrl-U clear · Left/Right Home/End\nEnter accept field · Esc discard edit";
    } else if wizard.review {
        let plan = yoctui_model::BitBakeSetupPlan::from_values(&wizard.values);
        lines.push(Line::from(format!(
            "Create: {}",
            plan.as_ref()
                .map(|p| p.destination().display().to_string())
                .unwrap_or_default()
        )));
        lines.push(Line::from(format!("Tool: {}", wizard.values[0])));
        lines.push(Line::from(format!(
            "Template: {}",
            if wizard.values[3].is_empty() {
                "choose in official wizard"
            } else {
                &wizard.values[3]
            }
        )));
        lines.push(Line::from(format!(
            "Registry: {}",
            if wizard.values[4].is_empty() {
                "tool default"
            } else {
                &wizard.values[4]
            }
        )));
        lines.push(Line::from(""));
        for text in [
            "Runs the official tool in a daemon-owned terminal.",
            "Choose Machine/Distro interactively there.",
            "Trusted templates may download sources and run setup scripts.",
            "Shared top-directory site.conf may be created by the tool.",
            "No image build or daemon retargeting starts automatically.",
            "Cancellation may leave partial files; Yoctui never deletes them.",
        ] {
            lines.push(Line::from(text));
        }
        hint = if wizard.preparing.is_some() {
            "Validating executable and destination…\nEsc cancels before launch"
        } else {
            "Enter confirm and start official setup wizard\nEsc back to fields"
        };
    } else {
        lines.push(Line::from(
            "Create a fresh setup, or use 2 Setups to import an existing one.",
        ));
        lines.push(Line::from(""));
        for (i, label) in BitBakeSetupWizard::LABELS.iter().enumerate() {
            if wizard.root_only && i != 1 {
                continue;
            }
            lines.push(Line::styled(
                format!("{} {label}", if i == wizard.field { ">" } else { " " }),
                if i == wizard.field {
                    selected_style(app, true)
                } else {
                    ThemePalette::for_app(app).base()
                },
            ));
            lines.push(Line::from(format!(
                "  {}",
                if wizard.values[i].is_empty() {
                    "(default / choose interactively)"
                } else {
                    &wizard.values[i]
                }
            )));
        }
        hint = "Tab/Shift-Tab field · Enter/e edit · s review/save\nEsc cancel · legacy/BSP profiles remain in 1 Environment";
    }
    frame.render_widget(
        Paragraph::new(lines).style(ThemePalette::for_app(app).base()),
        rows[0],
    );
    frame.render_widget(
        Paragraph::new(wizard.error.as_deref().unwrap_or_default())
            .style(severity_style(app, Severity::Warning))
            .wrap(Wrap { trim: false }),
        rows[1],
    );
    frame.render_widget(Paragraph::new(hint), rows[2]);
}

#[cfg(test)]
#[path = "tests/bitbake_setup_render.rs"]
mod tests;
