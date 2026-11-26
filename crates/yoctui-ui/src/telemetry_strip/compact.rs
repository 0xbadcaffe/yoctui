#[derive(Clone, Copy)]
pub(crate) enum TelemetryCell {
    Cpu,
    Ram,
    BuildFilesystem,
    Sstate,
    DiskRead,
    DiskWrite,
    NetworkReceive,
    NetworkTransmit,
    DiskIo,
}

pub(crate) fn render_telemetry_cell(
    frame: &mut Frame,
    app: &App,
    projection: &yoctui_model::HostTelemetryProjection,
    area: Rect,
    cell: TelemetryCell,
    divider: bool,
) {
    let block = Block::default().borders(if divider {
        Borders::RIGHT
    } else {
        Borders::NONE
    });
    let inner = block.inner(area);
    frame.render_widget(block, area);
    match cell {
        TelemetryCell::Cpu => render_cpu_telemetry_cell(frame, app, projection, inner),
        TelemetryCell::Ram => render_ram_telemetry_cell(frame, app, projection, inner),
        TelemetryCell::BuildFilesystem => {
            render_build_filesystem_telemetry_cell(frame, app, projection, inner);
        }
        TelemetryCell::Sstate => {
            let palette = ThemePalette::for_app(app);
            let style = if app.build.cache.summary.is_some_and(|summary| summary.valid()) {
                palette.base()
            } else {
                palette.role(palette.disabled, Modifier::DIM)
            };
            frame.render_widget(
                Paragraph::new(app.cache_status_lines()[0].clone())
                    .wrap(Wrap { trim: false })
                    .style(style),
                inner,
            );
        }
        TelemetryCell::DiskRead => {
            render_disk_io_projection(frame, app, projection, inner, Rect::default());
        }
        TelemetryCell::DiskWrite => {
            render_disk_io_projection(frame, app, projection, Rect::default(), inner);
        }
        TelemetryCell::NetworkReceive => {
            render_network_io_projection(frame, app, projection, inner, Rect::default());
        }
        TelemetryCell::NetworkTransmit => {
            render_network_io_projection(frame, app, projection, Rect::default(), inner);
        }
        TelemetryCell::DiskIo => {
            let rows = Layout::vertical([Constraint::Length(1); 2]).split(inner);
            render_disk_io_projection(frame, app, projection, rows[0], rows[1]);
        }
    }
}

pub(crate) fn render_compact_telemetry_strip(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Resources", false).style(palette.base());
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let telemetry = &app.host_telemetry;
    let values = [
        (
            "CPU",
            telemetry
                .cpu_utilization_percent
                .map(|value| value.min(100)),
        ),
        (
            "RAM",
            utilization_percent(
                telemetry.memory_total_bytes,
                telemetry.memory_available_bytes,
            ),
        ),
        (
            "FS",
            app.workspace.build_dir.as_ref().and_then(|_| {
                utilization_percent(telemetry.disk_total_bytes, telemetry.disk_available_bytes)
            }),
        ),
    ];
    let cells = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
    for ((label, sample), cell) in values.into_iter().zip(cells.iter()) {
        let warning = if label == "RAM" { MEMORY_WARNING_PERCENT } else { RESOURCE_WARNING_PERCENT };
        let text = sample.map_or_else(
            || format!("{label} {}", if cell.height > 1 { "--" } else { "n/a" }),
            |value| resource_meter_label(format!("{label} {value}%"), value, warning, cell.width),
        );
        let style = match (label, sample) {
            ("RAM", Some(percent)) => memory_meter_style(app, percent),
            ("CPU", Some(percent)) => cpu_meter_style(app, percent),
            (_, Some(percent)) => telemetry_meter_style(app, percent),
            (_, None) => palette.role(palette.disabled, Modifier::DIM),
        };
        frame.render_widget(
            Paragraph::new(text).style(style),
            Rect::new(cell.x, cell.y, cell.width, 1),
        );
        if cell.height < 2 {
            continue;
        }
        let bar = Rect::new(cell.x, cell.y + 1, cell.width.saturating_sub(1), 1);
        if let Some(percent) = sample {
            render_resource_track(frame, app, bar, percent, warning);
        } else {
            frame.render_widget(
                Paragraph::new("unavailable").style(palette.role(palette.muted, Modifier::DIM)),
                bar,
            );
        }
    }
}
