//! Native Ratatui tree and synchronized side-by-side text comparison.
use super::*;
use yoctui_model::{FileDiffCell, FileDiffPane};

pub(crate) fn workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let regions = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(0),
        Constraint::Length(2),
    ])
    .split(area);
    let state = &app.file_diff;
    let status = if state.comparing {
        "Comparing...".into()
    } else if let Some(diff) = &state.comparison {
        format!(
            "{} · {} changed rows · final newline L:{} R:{}",
            if diff.identical {
                "Files are identical"
            } else {
                "Files differ"
            },
            diff.changed_rows,
            if diff.final_newline[0] { "yes" } else { "no" },
            if diff.final_newline[1] { "yes" } else { "no" }
        )
    } else {
        "Mark one file per side with Space, then F5 to compare".into()
    };
    frame.render_widget(
        Paragraph::new(status).style(palette.base()).block(
            Block::default()
                .title("Diff tool · read-only")
                .borders(Borders::ALL),
        ),
        regions[0],
    );
    let panes = Layout::horizontal([Constraint::Length(area.width / 2), Constraint::Min(0)])
        .split(regions[1]);
    for (index, area) in panes.iter().enumerate() {
        if let Some(diff) = &state.comparison {
            comparison_pane(frame, app, index, *area, diff);
        } else {
            browser_pane(frame, app, index, *area, &state.panes[index]);
        }
    }
    let message = state.error.as_deref().unwrap_or(if state.comparison.is_some() {
        "Up/Down PgUp/PgDn Home/End scroll · Left/Right pan · Esc file trees"
    } else { "Tab side · Enter/Right expand · Left collapse · Backspace parent · . hidden · Esc navigator" });
    frame.render_widget(
        Paragraph::new(message)
            .wrap(Wrap { trim: false })
            .style(if state.error.is_some() {
                palette.role(palette.error, Modifier::BOLD)
            } else {
                palette.base()
            }),
        regions[2],
    );
}

fn browser_pane(frame: &mut Frame, app: &App, index: usize, area: Rect, pane: &FileDiffPane) {
    let palette = ThemePalette::for_app(app);
    let active = app.file_diff.active == index && app.focus == FocusTarget::Workspace;
    let title = format!(
        "{} · {}",
        if index == 0 { "Left" } else { "Right" },
        pane.browser.root.display()
    );
    let block = pane_block(app, &title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let regions = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(inner);
    let browser = &pane.browser;
    let viewport = yoctui_model::BoundedScroll::new(
        browser.selection,
        pane.offset,
        usize::from(regions[0].height),
        browser.entries.len(),
    );
    let mut lines = Vec::new();
    if pane.loading {
        lines.push(Line::raw("Loading directory..."));
    } else if let Some(error) = &pane.error {
        lines.push(Line::styled(
            error.clone(),
            palette.role(palette.error, Modifier::BOLD),
        ));
    } else if browser.entries.is_empty() {
        lines.push(Line::raw("No visible files · . shows hidden files"));
    } else {
        for row in viewport.visible_range() {
            let entry = &browser.entries[row];
            let marked = pane.marked.as_ref() == Some(&entry.path);
            let selected = row == browser.selection;
            let marker = if marked { "[*]" } else { "[ ]" };
            let branch = if entry.is_dir {
                if browser.expanded.contains(&entry.path) {
                    "v "
                } else {
                    "> "
                }
            } else {
                "  "
            };
            let name = entry.path.file_name().unwrap_or_default().to_string_lossy();
            let indent = "  ".repeat(entry.depth.min(32));
            let size = entry
                .size
                .filter(|_| !entry.is_dir)
                .map_or(String::new(), |s| format!("  {s} B"));
            let style = if marked {
                palette.role(palette.success, Modifier::BOLD)
            } else if entry.is_dir {
                palette.role(palette.accent, Modifier::BOLD)
            } else {
                palette.base()
            };
            let style = if selected {
                style
                    .add_modifier(Modifier::UNDERLINED)
                    .bg(if palette.attribute_only {
                        Color::Reset
                    } else {
                        palette.selection_background
                    })
            } else {
                style
            };
            lines.push(Line::styled(
                format!("{marker} {indent}{branch}{name}{size}"),
                style,
            ));
        }
    }
    frame.render_widget(Paragraph::new(lines).style(palette.base()), regions[0]);
    let selected = pane.marked.as_ref().map_or_else(
        || "Space: mark file".into(),
        |p| {
            format!(
                "Marked: {}",
                p.file_name().unwrap_or_default().to_string_lossy()
            )
        },
    );
    frame.render_widget(
        Paragraph::new(selected).style(palette.role(palette.success, Modifier::BOLD)),
        regions[1],
    );
}

fn comparison_pane(
    frame: &mut Frame,
    app: &App,
    index: usize,
    area: Rect,
    diff: &yoctui_model::FileDiffComparison,
) {
    let palette = ThemePalette::for_app(app);
    let title = format!(
        "{} · {} · {}",
        if index == 0 { "Left" } else { "Right" },
        diff.paths[index]
            .file_name()
            .unwrap_or_default()
            .to_string_lossy(),
        diff.paths[index].display()
    );
    let block = pane_block(
        app,
        &title,
        app.focus == FocusTarget::Workspace && app.file_diff.active == index,
    );
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let viewport = yoctui_model::BoundedScroll::new(
        app.file_diff.selection,
        app.file_diff.offset,
        usize::from(inner.height),
        diff.rows.len(),
    );
    let lines = viewport
        .visible_range()
        .map(|row| {
            let item = &diff.rows[row];
            let cell: Option<&FileDiffCell> = if index == 0 {
                item.left.as_ref()
            } else {
                item.right.as_ref()
            };
            let color = if item.equal {
                palette.primary_foreground
            } else if item.left.is_some() && item.right.is_some() {
                palette.warning
            } else if index == 0 {
                palette.error
            } else {
                palette.success
            };
            let style = palette.role(
                color,
                if item.equal {
                    Modifier::empty()
                } else {
                    Modifier::BOLD
                },
            );
            let style = if row == app.file_diff.selection {
                style.add_modifier(Modifier::UNDERLINED)
            } else {
                style
            };
            let text = cell.map_or(String::new(), |cell| {
                // Escape terminal controls; show tabs and CR rather than hiding differences.
                let safe = cell
                    .text
                    .chars()
                    .flat_map(|c| {
                        if c == '\t' {
                            "→   ".chars().collect::<Vec<_>>()
                        } else if c.is_control() {
                            c.escape_default().collect()
                        } else {
                            vec![c]
                        }
                    })
                    .skip(app.file_diff.horizontal)
                    .take(usize::from(inner.width.saturating_sub(8)))
                    .collect::<String>();
                format!(
                    "{:>5} {} {safe}",
                    cell.number,
                    if item.equal {
                        " "
                    } else if index == 0 {
                        "-"
                    } else {
                        "+"
                    }
                )
            });
            Line::styled(text, style)
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines).style(palette.base()), inner);
}
