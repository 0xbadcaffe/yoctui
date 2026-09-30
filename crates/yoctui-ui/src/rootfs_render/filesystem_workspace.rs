pub(crate) fn rootfs_filesystem_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let body = rootfs_workspace_shell(frame, app, area);
    if body.height == 0 {
        return;
    }
    if let Some(browser) = app.rootfs_browser() {
        layer_browser(frame, app, browser, body);
        return;
    }
    if let Some(lines) = rootfs_state_lines(app) {
        frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
        return;
    }
    let Some(composition) = app.rootfs_composition.composition() else {
        return;
    };
    let totals = composition.totals().0;
    let reason = if composition.root_directory.is_some() {
        "Press Enter to open the lazy IMAGE_ROOTFS tree (or retry after a read failure)."
    } else {
        "The exact IMAGE_ROOTFS directory is unavailable; refresh with r after rebuilding the image."
    };
    let mut lines = vec![
        Line::from(format!(
            "Filesystem authority · {} exact bytes",
            totals.filesystem_bytes
        )),
        Line::from(format!(
            "files {} · dirs {} · symlinks {} · special {}",
            totals.files, totals.directories, totals.symlinks, totals.other
        )),
        Line::from(reason),
        Line::from("Installed-package evidence remains separate; press Shift-Tab."),
    ];
    if let yoctui_model::RootfsAuthority::Unavailable { reason } = &composition.filesystem_tree {
        lines.push(Line::from(format!(
            "Filesystem inventory unavailable: {reason}"
        )));
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), body);
}
