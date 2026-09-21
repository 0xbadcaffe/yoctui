pub(crate) fn rootfs_filesystem_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if body.height == 0 {
        return;
    }
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let Some(tree) = composition.filesystem_tree() else {
        let reason = match &composition.filesystem_tree {
            yoctui_model::RootfsAuthority::Unavailable { reason } => reason.as_str(),
            _ => "filesystem authority is unavailable",
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Filesystem tree unavailable\n{reason}\n\nInstalled-package evidence remains separate; press Shift-Tab."
            ))
            .wrap(Wrap { trim: false }),
            body,
        );
        return;
    };
    let totals = composition.totals().0;
    let selected = app
        .rootfs_entry_selection
        .as_ref()
        .and_then(|identity| {
            tree.entries
                .iter()
                .position(|entry| &entry.identity == identity)
        })
        .unwrap_or(0);
    let header_rows = 4_usize;
    let visible = usize::from(body.height).saturating_sub(header_rows).max(1);
    let start = selected
        .saturating_sub(visible / 2)
        .min(tree.entries.len().saturating_sub(visible));
    let mut lines = vec![
        Line::from(format!(
            "Filesystem authority · {} entries · {} exact bytes",
            tree.entries.len(),
            totals.filesystem_bytes
        )),
        Line::from(format!(
            "files {} · dirs {} · symlinks {} · special {} · j/k navigate · r refresh",
            totals.files, totals.directories, totals.symlinks, totals.other
        )),
        Line::from(format!(
            "Showing {}–{} of {} · package ownership is independent authority",
            start.saturating_add(1).min(tree.entries.len()),
            start.saturating_add(visible).min(tree.entries.len()),
            tree.entries.len()
        )),
        Line::from(
            "Logical path                                      Kind          Exact bytes   Package",
        ),
    ];
    for entry in tree.entries.iter().skip(start).take(visible) {
        let is_selected = app.rootfs_entry_selection.as_ref() == Some(&entry.identity);
        let indent = "  ".repeat(entry.identity.depth().saturating_sub(1).min(12));
        let branch = if app.theme == Theme::Monochrome
            || app.preferences.symbols == SymbolPreference::Ascii
        {
            "|-"
        } else {
            "├─"
        };
        let name = if entry.identity.0 == std::path::Path::new("/") {
            "/".into()
        } else {
            entry
                .identity
                .0
                .file_name()
                .map_or_else(|| "?".into(), |name| name.to_string_lossy().into_owned())
        };
        let kind = match entry.kind {
            RootfsEntryKind::Directory => "directory",
            RootfsEntryKind::RegularFile => "file",
            RootfsEntryKind::Symlink => "symlink",
            RootfsEntryKind::Other => "special",
        };
        lines.push(
            Line::from(format!(
                "{:<49} {:<12} {:>11}   {}",
                format!("{indent}{branch} {name}"),
                kind,
                entry.size_bytes,
                entry
                    .package
                    .as_ref()
                    .map_or("unavailable", |value| value.name.as_str())
            ))
            .style(selected_style(app, is_selected)),
        );
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
}
