use super::*;

pub(super) fn files(frame: &mut Frame, app: &App, area: Rect) {
    let Some(s) = app.workbench.current() else {
        return;
    };
    let palette = ThemePalette::for_app(app);
    let mut lines = Vec::new();
    let explorer = yoctui_app::workbench_explorer_viewport(s, area.height);
    let folders = explorer
        .as_ref()
        .map_or(0..0, |viewport| viewport.folders.clone());
    for index in folders {
        let folder = &s.folders[index];
        let selected = index == s.folder;
        lines.push(Line::styled(
            format!(
                "{} {}{}",
                if selected { "▾" } else { "▸" },
                folder
                    .browser
                    .root
                    .file_name()
                    .unwrap_or(folder.browser.root.as_os_str())
                    .to_string_lossy(),
                if folder.loading { " · loading…" } else { "" }
            ),
            if selected {
                palette.role(palette.informational, Modifier::BOLD)
            } else {
                Style::default()
            },
        ));
        if !selected {
            continue;
        }
        lines.push(Line::from(file_browser_breadcrumb(
            &folder.browser,
            area.width.saturating_sub(2),
        )));
        if let Some(error) = &folder.error {
            lines.push(Line::styled(
                error.clone(),
                palette.role(palette.warning, Modifier::empty()),
            ));
        }
        for row in explorer.as_ref().unwrap().files.visible_range() {
            let entry = &folder.browser.entries[row];
            let prefix = if entry.is_dir {
                if folder.browser.expanded.contains(&entry.path) {
                    "▾ "
                } else {
                    "▸ "
                }
            } else {
                "  "
            };
            lines.push(Line::styled(
                format!(
                    "  {}{prefix}{}",
                    "  ".repeat(entry.depth.min(32)),
                    entry.path.file_name().unwrap_or_default().to_string_lossy()
                ),
                selection_style(
                    app,
                    row == folder.browser.selection,
                    s.focus == Focus::Files && pane_has_focus(app, FocusTarget::Workspace),
                ),
            ));
        }
    }
    if s.folders.is_empty() {
        lines.push(Line::from("Alt+a Add folder"));
    }
    let range = explorer
        .as_ref()
        .map_or_else(|| "0/0".into(), |viewport| viewport.files.range_label());
    let label = if range.len() + "Explorer ".len() <= usize::from(area.width.saturating_sub(2)) {
        "Explorer"
    } else {
        "Files"
    };
    let title = format!("{label} {range}");
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title)),
        area,
    );
    if let Some(explorer) = explorer {
        let scroll = explorer.files;
        let fixed_rows = (s.folder.saturating_sub(explorer.folders.start) as u16).saturating_add(2)
            + u16::from(s.folders[s.folder].error.is_some());
        let list_area = Rect::new(
            area.x,
            area.y.saturating_add(fixed_rows),
            area.width,
            (scroll.viewport as u16).saturating_add(2),
        );
        render_file_browser_scrollbar(frame, app, list_area, scroll);
    }
}
