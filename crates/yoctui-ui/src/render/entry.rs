pub fn render(frame: &mut Frame, app: &App) {
    render_at(frame, app, SystemTime::now());
}

/// Render one frame with an injected clock.
///
/// Production uses [`render`]. Deterministic visual tests and the release
/// profiling workload use this entry point so clock changes cannot alter the
/// rendered cell buffer between otherwise identical frames.
pub(crate) fn selected_table_viewport(
    selection: usize,
    total: usize,
    area: Rect,
) -> std::ops::Range<usize> {
    yoctui_model::centered_viewport_range(
        (total > 0).then_some(selection),
        total,
        usize::from(area.height.saturating_sub(3)).max(1),
    )
}

pub fn render_at(frame: &mut Frame, app: &App, now: SystemTime) {
    let area = frame.area();
    let palette = ThemePalette::for_app(app);
    frame.render_widget(Block::default().style(palette.base()), area);
    if area.width < 80 || area.height < 24 {
        frame.render_widget(
            Paragraph::new(format!(
                "Yoctui needs at least 80x24.\nCurrent terminal: {}x{}.\nResize the terminal or press Q to quit.",
                area.width, area.height
            ))
            .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }
    let [header, footer] = yoctui_app::workbench_chrome_heights(app, area.width, area.height);
    let chunks = Layout::vertical([
        Constraint::Length(header),
        Constraint::Min(1),
        Constraint::Length(footer),
    ])
    .split(area);
    workbench_header(frame, app, chunks[0], now);
    responsive_shell(frame, app, chunks[1], area.width, now);
    workbench_footer(frame, app, chunks[2], now);
    let screen_area = area;
    let _overlay_rendered = render_shell_overlays(frame, app, area)
        || render_workflow_dialogs(frame, app, area)
        || render_project_dialogs(frame, app, area)
        || render_development_dialogs(frame, app, area)
        || render_setup_dialogs(frame, app, area);
    if !app.command_palette_open
        && let Some(dialog) = app.active_dialog()
    {
        dialog_compatibility_overlay(frame, app, dialog, screen_area);
    }
    if popup_notification(app).is_some()
        && app.active_dialog().is_none()
        && !app.menu.is_open()
        && !app.onboarding.open
        && !app.keymap_preferences_ui.open
        && !app.command_palette_open
        && !(app.screen == Screen::RawMode
            && matches!(
                app.raw_mode.view,
                yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
            ))
    {
        notification_popup(frame, app, screen_area);
    }
}
