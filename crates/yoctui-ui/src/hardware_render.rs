//! Persistent hardware-document library, browser, and embedded viewer.

use super::*;

pub(super) fn hardware_workspace(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(viewer) = app.hardware.viewer.as_ref() {
        render_viewer(frame, app, area, viewer);
    } else if let Some(browser) = app.hardware.browser.as_ref() {
        render_browser(frame, app, area, browser);
    } else {
        render_library(frame, app, area);
    }
    if let Some(document) = app.hardware.removal_pending.as_ref() {
        render_remove_confirmation(frame, app, area, document);
    }
}

fn render_library(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let rows = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);
    let categories = HardwareCategory::ALL
        .iter()
        .map(|category| {
            Span::styled(
                format!(" {} ", category.label()),
                if *category == app.hardware.category {
                    palette.selected().add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(palette.secondary_foreground)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(categories)).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Hardware · document library ")
                .border_style(Style::default().fg(palette.focused_border)),
        ),
        rows[0],
    );

    let documents = app.hardware.visible_documents();
    let capacity = usize::from(rows[1].height.saturating_sub(2)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        (!documents.is_empty()).then_some(app.hardware.selection),
        documents.len(),
        capacity,
    );
    let lines = if documents.is_empty() {
        vec![Line::from(
            "No documents in this category. Press a to browse and add one.",
        )]
    } else {
        documents[viewport.clone()]
            .iter()
            .enumerate()
            .map(|(offset, document)| {
                let selected = offset + viewport.start == app.hardware.selection;
                Line::from(vec![
                    Span::styled(
                        if selected { "▶ " } else { "  " },
                        Style::default().fg(palette.accent),
                    ),
                    Span::styled(
                        format!(
                            "{:<18}",
                            if app.hardware.document_is_missing(document) {
                                "Missing"
                            } else {
                                document.kind.label()
                            }
                        ),
                        Style::default().fg(palette.informational),
                    ),
                    Span::styled(
                        document.name(),
                        if selected {
                            palette.selected()
                        } else {
                            Style::default()
                        },
                    ),
                ])
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {} · {} document{} ",
                    app.hardware.category.label(),
                    documents.len(),
                    if documents.len() == 1 { "" } else { "s" }
                ))
                .border_style(Style::default().fg(palette.inactive_border)),
        ),
        rows[1],
    );
    let detail = app.hardware.selected_document().map_or_else(
        || "Supported: PDF, KiCad .kicad_sch/.sch, SVG, PNG, JPEG, GIF, BMP, TIFF, WebP".into(),
        |document| document.path.display().to_string(),
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(detail),
            Line::from("←/→ category  ↑/↓ select  Enter view  a add  d remove  r reload  F12 menu"),
        ])
        .style(Style::default().fg(palette.secondary_foreground)),
        rows[2],
    );
}

fn render_browser(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    browser: &yoctui_model::HardwareBrowserState,
) {
    let palette = ThemePalette::for_app(app);
    let rows = Layout::vertical([
        Constraint::Length(4),
        Constraint::Min(3),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!(
            "Directory: {}\nAdd to: {}   ←/→ changes category",
            browser.directory.display(),
            browser.category.label()
        ))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Add hardware document ")
                .border_style(Style::default().fg(palette.focused_border)),
        ),
        rows[0],
    );
    let capacity = usize::from(rows[1].height.saturating_sub(2)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        (!browser.entries.is_empty()).then_some(browser.selection),
        browser.entries.len(),
        capacity,
    );
    let mut lines = browser.entries[viewport.clone()]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let selected = offset + viewport.start == browser.selection;
            let kind = if entry.is_directory {
                "DIR"
            } else {
                entry.kind.map_or("?", |kind| kind.label())
            };
            Line::from(vec![
                Span::styled(
                    format!("{kind:<18}"),
                    Style::default().fg(palette.informational),
                ),
                Span::styled(
                    entry.name.clone(),
                    if selected {
                        palette.selected()
                    } else {
                        Style::default()
                    },
                ),
            ])
        })
        .collect::<Vec<_>>();
    if browser.loading {
        lines.insert(
            0,
            Line::styled(
                "⠋ Loading directory…",
                palette.role(palette.running, Modifier::BOLD),
            ),
        );
    } else if let Some(error) = browser.error.as_ref() {
        lines.insert(
            0,
            Line::styled(error, palette.role(palette.error, Modifier::BOLD)),
        );
    } else if lines.is_empty() {
        lines.push(Line::from("No supported documents or directories."));
    }
    frame.render_widget(
        Paragraph::new(lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Files ")
                .border_style(Style::default().fg(palette.inactive_border)),
        ),
        rows[1],
    );
    frame.render_widget(
        Paragraph::new("Enter open/add  Backspace parent  ←/→ category  a add file  Esc cancel")
            .style(Style::default().fg(palette.secondary_foreground)),
        rows[2],
    );
}

fn render_viewer(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewer: &yoctui_model::HardwareViewerState,
) {
    let palette = ThemePalette::for_app(app);
    let rows = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(2),
        Constraint::Length(2),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(format!(
            "{} · {} · page {}/{} · zoom {}%",
            viewer.document.name(),
            viewer.document.kind.label(),
            viewer.page,
            viewer.page_count,
            viewer.zoom_percent
        ))
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_style(Style::default().fg(palette.focused_border)),
        )
        .style(palette.role(palette.heading, Modifier::BOLD)),
        rows[0],
    );
    if viewer.loading {
        frame.render_widget(
            Paragraph::new("⠋ Loading hardware document…")
                .style(palette.role(palette.running, Modifier::BOLD)),
            rows[1],
        );
    } else if let Some(error) = viewer.error.as_ref() {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .wrap(Wrap { trim: false })
                .style(palette.role(palette.error, Modifier::BOLD)),
            rows[1],
        );
    } else if let Some(preview) = viewer.preview.as_ref() {
        match preview {
            yoctui_model::HardwarePreview::Text { lines, limitation } => {
                render_text_preview(frame, app, rows[1], viewer, lines, limitation.as_deref())
            }
            yoctui_model::HardwarePreview::Raster(raster) => {
                render_raster_preview(frame, rows[1], viewer, raster)
            }
        }
    }
    let search = if viewer.searching {
        format!("Search: {}_", viewer.query)
    } else if viewer.query.is_empty() {
        String::new()
    } else {
        format!(
            "Search: {} · {}/{}",
            viewer.query,
            viewer
                .match_selection
                .saturating_add(1)
                .min(viewer.matches.len()),
            viewer.matches.len()
        )
    };
    frame.render_widget(Paragraph::new(vec![
        Line::from("Esc library  PgUp/PgDn page  +/- zoom  0 fit  arrows/hjkl pan  / search  n/N match  r reload"),
        Line::styled(search, Style::default().fg(palette.accent)),
    ]), rows[2]);
}

fn render_text_preview(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewer: &yoctui_model::HardwareViewerState,
    lines: &[String],
    limitation: Option<&str>,
) {
    let palette = ThemePalette::for_app(app);
    let capacity = usize::from(area.height).saturating_sub(usize::from(limitation.is_some()));
    let mut visible = lines
        .iter()
        .skip(viewer.pan_y)
        .take(capacity)
        .enumerate()
        .map(|(offset, line)| {
            let text = line.chars().skip(viewer.pan_x).collect::<String>();
            let matched = viewer.matches.contains(&(viewer.pan_y + offset));
            Line::styled(
                text,
                if matched {
                    palette.role(palette.warning, Modifier::BOLD)
                } else {
                    Style::default().fg(palette.primary_foreground)
                },
            )
        })
        .collect::<Vec<_>>();
    if let Some(limitation) = limitation {
        visible.insert(
            0,
            Line::styled(limitation, Style::default().fg(palette.warning)),
        );
    }
    frame.render_widget(Paragraph::new(visible).wrap(Wrap { trim: false }), area);
}

fn render_raster_preview(
    frame: &mut Frame,
    area: Rect,
    viewer: &yoctui_model::HardwareViewerState,
    raster: &yoctui_model::HardwareRaster,
) {
    let zoom = usize::from(viewer.zoom_percent).max(1);
    let lines = (0..usize::from(area.height))
        .map(|row| {
            let y_top = viewer.pan_y + row.saturating_mul(200) / zoom;
            let y_bottom = viewer.pan_y + (row.saturating_mul(2) + 1).saturating_mul(100) / zoom;
            Line::from(
                (0..usize::from(area.width))
                    .map(|column| {
                        let x = viewer.pan_x + column.saturating_mul(100) / zoom;
                        let top = pixel(raster, x, y_top);
                        let bottom = pixel(raster, x, y_bottom);
                        Span::styled(
                            "▀",
                            Style::default()
                                .fg(Color::Rgb(top.red, top.green, top.blue))
                                .bg(Color::Rgb(bottom.red, bottom.green, bottom.blue)),
                        )
                    })
                    .collect::<Vec<_>>(),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

fn pixel(raster: &yoctui_model::HardwareRaster, x: usize, y: usize) -> yoctui_model::HardwareRgb {
    if x < raster.width && y < raster.height {
        raster.pixels[y * raster.width + x]
    } else {
        yoctui_model::HardwareRgb {
            red: 0,
            green: 0,
            blue: 0,
        }
    }
}

fn render_remove_confirmation(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    document: &yoctui_model::HardwareDocument,
) {
    let palette = ThemePalette::for_app(app);
    let popup = bounded_dialog_rect(area, 72, 7);
    frame.render_widget(Clear, popup);
    frame.render_widget(Paragraph::new(format!("Remove {} from the Hardware library?\n\nEnter/y confirm   Esc/n cancel\nThe source file will not be deleted.", document.name()))
        .wrap(Wrap { trim: false }).block(Block::default().borders(Borders::ALL)
        .title(" Confirm removal ").border_style(Style::default().fg(palette.warning))), popup);
}
