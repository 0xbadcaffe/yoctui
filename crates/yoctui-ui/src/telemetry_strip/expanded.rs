pub(crate) fn render_telemetry_strip(frame: &mut Frame, app: &App, area: Rect) {
    let mode = telemetry_strip_mode(area);
    if mode == TelemetryStripMode::Hidden {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let projection = app.host_telemetry_projection();
    let block = pane_block(
        app,
        "Resource Telemetry · bounded 60-sample histories",
        false,
    )
    .style(palette.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut cells = vec![
        TelemetryCell::Cpu,
        TelemetryCell::Ram,
        TelemetryCell::BuildFilesystem,
    ];
    if area.width == 86 {
        cells.push(TelemetryCell::Sstate);
    } else {
        match mode {
            TelemetryStripMode::Wide => {
                if disk_io_supported(&projection) {
                    cells.extend([TelemetryCell::DiskRead, TelemetryCell::DiskWrite]);
                }
                if network_io_supported(&projection) {
                    cells.extend([
                        TelemetryCell::NetworkReceive,
                        TelemetryCell::NetworkTransmit,
                    ]);
                }
            }
            TelemetryStripMode::Medium if disk_io_supported(&projection) => {
                cells.push(TelemetryCell::DiskIo);
            }
            TelemetryStripMode::Medium | TelemetryStripMode::Hidden => {}
        }
    }
    let count = u32::try_from(cells.len()).unwrap_or(1);
    let areas = Layout::horizontal(vec![Constraint::Ratio(1, count); cells.len()]).split(inner);
    for (index, (cell, area)) in cells.into_iter().zip(areas.iter().copied()).enumerate() {
        render_telemetry_cell(frame, app, &projection, area, cell, index + 1 < areas.len());
    }
}

pub(crate) fn render_expanded_telemetry(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let block = Block::default()
        .title("Telemetry charts · latest 60 valid samples · missing samples are not zero")
        .borders(Borders::ALL)
        .style(palette.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    let projection = app.host_telemetry_projection();
    let has_filesystem = app.workspace.build_dir.is_some();
    let row_count = projection.series.len() + usize::from(has_filesystem);
    let denominator = u32::try_from(row_count).unwrap_or(1).max(1);
    let rows = Layout::vertical(vec![Constraint::Ratio(1, denominator); row_count]).split(inner);
    let mut row_index = 0;
    if has_filesystem {
        render_disk_gauge(frame, app, rows[row_index]);
        row_index += 1;
    }
    let options = WidgetRenderOptions {
        unicode: app.preferences.symbols == SymbolPreference::Unicode
            && app.preferences.charts == yoctui_model::ChartPreference::Automatic,
        reduced_motion: app.reduced_motion,
    };
    for (series, row) in projection
        .series
        .iter()
        .zip(rows.iter().copied().skip(row_index))
    {
        render_history_chart(
            frame,
            row,
            &series.history,
            palette.widget_styles(),
            options,
        );
    }
}

pub(crate) fn render_tasks_context_zoom(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    if area.height < 12 {
        render_expanded_telemetry(frame, app, area);
        return;
    }
    let history_height = area.height.div_ceil(4).clamp(5, 10);
    let rows =
        Layout::vertical([Constraint::Length(history_height), Constraint::Min(1)]).split(area);
    render_job_history(frame, app, rows[0], now);
    render_expanded_telemetry(frame, app, rows[1]);
}
