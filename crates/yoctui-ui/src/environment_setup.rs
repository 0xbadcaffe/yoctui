use super::*;
use yoctui_model::EnvironmentSetup;

pub(super) fn environment_setup_popup(
    frame: &mut Frame,
    app: &App,
    setup: &EnvironmentSetup,
    area: Rect,
) {
    let width = area.width.saturating_sub(4).min(110);
    let height = area.height.saturating_sub(4).min(28);
    let popup = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let title = if setup.browser.is_some() {
        if setup.field == 2 {
            "Browse environment scripts"
        } else {
            "Browse directories"
        }
    } else {
        "Configure build environment"
    };
    frame.render_widget(dialog_block(app, title, DialogTone::Standard), popup);
    let inner = popup.inner(Margin::new(1, 1));
    if inner.height < 4 || inner.width < 8 {
        return;
    }
    let rows = Layout::vertical([
        Constraint::Min(1),
        Constraint::Length(2),
        Constraint::Length(2),
    ])
    .split(inner);
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    let hint;
    let mut notice = setup.error.clone();
    if let Some(browser) = &setup.browser {
        lines.push(Line::from(format!(
            "Choosing: {} (local filesystem)",
            EnvironmentSetup::LABELS[setup.field]
        )));
        if let Some(directory) = &browser.directory {
            lines.push(Line::from(directory.path.display().to_string()));
            let capacity = usize::from(rows[0].height.saturating_sub(3)).max(1);
            let start = browser.selection.saturating_sub(capacity.saturating_sub(1));
            let files = if setup.field == 2 {
                directory.files.as_slice()
            } else {
                &[]
            };
            if directory.children.is_empty() && files.is_empty() {
                lines.push(Line::from(if setup.field == 2 {
                    "No files or directories. Left goes to parent."
                } else {
                    "No child directories. s uses this directory."
                }));
            }
            for (index, path) in directory
                .children
                .iter()
                .chain(files.iter())
                .enumerate()
                .skip(start)
                .take(capacity)
            {
                let label = path.file_name().unwrap_or_default().to_string_lossy();
                let style = if index == browser.selection {
                    selected_style(app, true)
                } else {
                    palette.base()
                };
                lines.push(Line::styled(
                    format!(
                        "{} {label}{}",
                        if index == browser.selection { ">" } else { " " },
                        if index < directory.children.len() {
                            "/"
                        } else {
                            ""
                        }
                    ),
                    style,
                ));
            }
            if notice.is_none() {
                notice = directory.notice.clone();
            }
        }
        if browser.loading {
            notice = Some(if setup.field == 2 {
                "Loading files... Esc cancels".into()
            } else {
                "Loading directories... Esc cancels".into()
            });
        }
        hint = if setup.field == 2 {
            "Up/Down select  Enter open folder/choose file  Left/Backspace parent\nPgUp/PgDn Home/End  s choose file  Esc back"
        } else {
            "Up/Down select  Enter/Right open  Left/Backspace parent\nPgUp/PgDn Home/End  s use this directory  Esc back"
        };
    } else if let Some(editor) = &setup.editor {
        lines.push(Line::from(format!(
            "Edit {}",
            EnvironmentSetup::LABELS[setup.field]
        )));
        lines.push(Line::from(
            "Absolute path; the selected value is replaced when you type.",
        ));
        lines.push(Line::from(""));
        // Keep the cursor and the end of long paths visible without mutating state.
        let before = &editor.text[..editor.cursor];
        let available = usize::from(rows[0].width.saturating_sub(2));
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
        let style = if editor.selection.is_some() {
            selected_style(app, true)
        } else {
            palette.base()
        };
        lines.push(Line::from(vec![
            Span::styled(suffix, style),
            Span::styled("|", Style::default().add_modifier(Modifier::REVERSED)),
            Span::styled(after, style),
        ]));
        hint = "Type/paste path  Left/Right Home/End  Ctrl-U clear\nEnter accept value  Esc discard edit";
    } else {
        lines.push(Line::from(
            "Save opens a daemon connection review; confirm to initialize and refresh.",
        ));
        lines.push(Line::from(
            "No daemon or initialized build environment is needed to browse.",
        ));
        lines.push(Line::from(""));
        for (index, label) in EnvironmentSetup::LABELS.iter().enumerate() {
            let style = if index == setup.field {
                selected_style(app, true)
            } else {
                palette.base()
            };
            lines.push(Line::styled(
                format!(
                    "{} {label}: {}",
                    if index == setup.field { ">" } else { " " },
                    if setup.values[index].is_empty() {
                        "(not set)"
                    } else {
                        &setup.values[index]
                    }
                ),
                style,
            ));
        }
        lines.push(Line::from(""));
        lines.push(Line::from(
            "Source/Build: b browse folders. Script: b browse and choose a file.",
        ));
        lines.push(Line::from(
            "e types any path. Choosing Source also detects its default script.",
        ));
        hint = "Tab/Shift-Tab Up/Down field  Enter/e edit  b browse\ns save profile and review connection  Esc cancel";
    }
    frame.render_widget(Paragraph::new(lines).style(palette.base()), rows[0]);
    frame.render_widget(
        Paragraph::new(notice.unwrap_or_default())
            .style(severity_style(app, yoctui_model::Severity::Warning))
            .wrap(Wrap { trim: false }),
        rows[1],
    );
    frame.render_widget(Paragraph::new(hint).style(palette.base()), rows[2]);
}

#[cfg(test)]
#[path = "tests/environment_setup.rs"]
mod tests;
