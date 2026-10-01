use super::*;
use ratatui::widgets::Gauge;
use yoctui_model::{HARDWARE_BRINGUP_STAGES, HardwareProjectForm};

pub(super) fn render(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.hardware.projects;
    let palette = ThemePalette::for_app(app);
    let rows = Layout::vertical([
        Constraint::Length(if state.project.is_some() { 5 } else { 3 }),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);
    if let Some(project) = &state.project {
        let progress = project
            .progress
            .iter()
            .enumerate()
            .map(|(stage, value)| format!("{} {}%", HARDWARE_BRINGUP_STAGES[stage], value))
            .collect::<Vec<_>>()
            .join(" · ");
        frame.render_widget(
            Paragraph::new(format!(
                "{}\n{}",
                project.root.join(&state.relative).display(),
                progress
            ))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} · manual bring-up ", project.name)),
            ),
            rows[0],
        );
        let gauge = Rect::new(
            rows[0].x.saturating_add(1),
            rows[0].y.saturating_add(3),
            rows[0].width.saturating_sub(2),
            rows[0].height.saturating_sub(3).min(1),
        );
        frame.render_widget(
            Gauge::default()
                .percent(project.percent())
                .label(format!("User-reported bring-up {}%", project.percent()))
                .gauge_style(Style::default().fg(palette.accent)),
            gauge,
        );
    } else {
        frame.render_widget(
            Paragraph::new(
                "Persistent project folders · n New project · Enter open · p Document library",
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Hardware · Projects "),
            ),
            rows[0],
        );
    }
    let capacity = usize::from(rows[1].height.saturating_sub(2)).max(1);
    let (count, title) = if state.project.is_some() {
        (state.entries.len(), " Project files · folders first ")
    } else {
        (state.catalog.len(), " Projects ")
    };
    let viewport = yoctui_model::centered_viewport_range(
        (count > 0).then_some(state.selection),
        count,
        capacity,
    );
    let mut lines = viewport
        .map(|index| {
            let text = if state.project.is_some() {
                let entry = &state.entries[index];
                format!(
                    "{} {:<20} {:>10} B  {}",
                    if index == state.selection { "▶" } else { " " },
                    if entry.is_directory {
                        "DIR"
                    } else {
                        entry.kind.map_or("Stored only", |kind| kind.label())
                    },
                    if entry.is_directory { 0 } else { entry.size },
                    entry.name
                )
            } else {
                let project = &state.catalog[index];
                format!(
                    "{} {:3}%  {}",
                    if index == state.selection { "▶" } else { " " },
                    project.percent(),
                    project.name
                )
            };
            Line::styled(
                text,
                if index == state.selection {
                    palette.selected()
                } else {
                    Style::default().fg(palette.primary_foreground)
                },
            )
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        lines.push(Line::from(if state.project.is_some() {
            "Empty folder. n creates a folder; a imports any regular file."
        } else {
            "No projects. Press n to create one."
        }));
    }
    if state.loading {
        lines.insert(0, Line::from("Loading/saving Hardware project…"));
    }
    frame.render_widget(
        Paragraph::new(lines).block(Block::default().borders(Borders::ALL).title(title)),
        rows[1],
    );
    frame.render_widget(Paragraph::new(vec![
        Line::styled(state.error.as_deref().unwrap_or("Viewing: TXT, PDF, KiCad, Altium/Xpedition schematics; other files are stored only."), Style::default().fg(if state.error.is_some() { palette.error } else { palette.secondary_foreground })),
        Line::from("↑/↓ select  Enter open  Backspace parent  n new project/folder  a import  s bring-up  r refresh  p library"),
    ]).wrap(Wrap { trim: false }), rows[2]);
    if let Some((directory, entries, selection)) = &state.import_browser {
        let popup = bounded_dialog_rect(area, 100, 22);
        frame.render_widget(Clear, popup);
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Import any file · Enter directory/copy · Backspace parent · Esc cancel ");
        let inner = block.inner(popup);
        frame.render_widget(block, popup);
        let capacity = usize::from(inner.height.saturating_sub(3)).max(1);
        let viewport = yoctui_model::centered_viewport_range(
            (!entries.is_empty()).then_some(*selection),
            entries.len(),
            capacity,
        );
        let mut lines = vec![
            Line::from(directory.display().to_string()),
            Line::from(state.error.as_deref().unwrap_or(if state.loading {
                "Loading/importing…"
            } else {
                "Source files are copied, not moved. Existing names are never overwritten."
            })),
        ];
        lines.extend(viewport.map(|index| {
            Line::styled(
                format!(
                    "{:<5} {}",
                    if entries[index].is_directory {
                        "DIR"
                    } else {
                        "FILE"
                    },
                    entries[index].name
                ),
                if index == *selection {
                    palette.selected()
                } else {
                    Style::default()
                },
            )
        }));
        frame.render_widget(Paragraph::new(lines), inner);
    }
    if let Some(form) = &state.form {
        render_form(frame, app, area, form);
    }
}

fn render_form(frame: &mut Frame, app: &App, area: Rect, form: &HardwareProjectForm) {
    let state = &app.hardware.projects;
    let palette = ThemePalette::for_app(app);
    let (title, mut lines) = match form {
        HardwareProjectForm::Name { value } => (
            if state.project.is_some() {
                " Create project subfolder "
            } else {
                " Create Hardware project "
            },
            vec![
                Line::from(format!("Name: {value}_")),
                Line::from("A real folder is created; existing names are not overwritten."),
                Line::from("Enter create · Esc cancel"),
            ],
        ),
        HardwareProjectForm::Progress {
            values,
            stage,
            digits,
        } => {
            let mut lines = vec![Line::from(
                "Manual/user-reported progress · independent of BitBake",
            )];
            lines.extend(
                HARDWARE_BRINGUP_STAGES
                    .iter()
                    .enumerate()
                    .map(|(index, label)| {
                        Line::styled(
                            format!(
                                "{} {:<14} {:3}% {}",
                                if index == *stage { "▶" } else { " " },
                                label,
                                values[index],
                                if index == *stage && !digits.is_empty() {
                                    format!("({digits}_) ")
                                } else {
                                    String::new()
                                }
                            ),
                            if index == *stage {
                                palette.selected()
                            } else {
                                Style::default()
                            },
                        )
                    }),
            );
            lines.push(Line::from(
                "↑/↓/Tab stage · ←/→ ±5 · digits exact 0–100 · Space 0/100",
            ));
            lines.push(Line::from("Enter save · Esc cancel"));
            (" Manual project bring-up ", lines)
        }
    };
    if state.loading {
        lines.push(Line::from("Saving…"));
    }
    if let Some(error) = &state.error {
        lines.push(Line::styled(error, Style::default().fg(palette.error)));
    }
    let popup = bounded_dialog_rect(area, 100, 15);
    frame.render_widget(Clear, popup);
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(Style::default().fg(palette.focused_border)),
        ),
        popup,
    );
}
