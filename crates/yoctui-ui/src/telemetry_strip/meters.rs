#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TelemetryStripMode {
    Wide,
    Medium,
    Hidden,
}

pub(crate) fn telemetry_strip_mode(area: Rect) -> TelemetryStripMode {
    if area.height < 4 || area.width < 64 {
        TelemetryStripMode::Hidden
    } else if area.width < 112 {
        TelemetryStripMode::Medium
    } else {
        TelemetryStripMode::Wide
    }
}

pub(crate) fn disk_io_supported(projection: &yoctui_model::HostTelemetryProjection) -> bool {
    projection
        .series(TelemetryMetric::DiskReadRate)
        .is_supported()
        || projection
            .series(TelemetryMetric::DiskWriteRate)
            .is_supported()
}

pub(crate) fn network_io_supported(projection: &yoctui_model::HostTelemetryProjection) -> bool {
    projection
        .series(TelemetryMetric::NetworkReceiveRate)
        .is_supported()
        || projection
            .series(TelemetryMetric::NetworkTransmitRate)
            .is_supported()
}

pub(crate) struct DenseTelemetryMeter<'a> {
    pub(crate) title: &'a str,
    pub(crate) percent: u8,
    pub(crate) detail: &'a str,
    pub(crate) active_style: Style,
    pub(crate) history: &'a [u64],
}

pub(crate) fn render_dense_telemetry_meter(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    meter: DenseTelemetryMeter<'_>,
) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let DenseTelemetryMeter {
        title,
        percent,
        detail,
        active_style,
        history,
    } = meter;
    let percent = percent.min(100);
    let percent_text = format!("{percent:>3}%");
    let title_width = area.width.saturating_sub(5);
    frame.render_widget(
        Paragraph::new(title).style(palette.role(palette.informational, Modifier::BOLD)),
        Rect::new(area.x, area.y, title_width, 1),
    );
    frame.render_widget(
        Paragraph::new(percent_text)
            .alignment(Alignment::Right)
            .style(active_style.add_modifier(Modifier::BOLD)),
        Rect::new(area.x + title_width, area.y, area.width - title_width, 1),
    );

    let detail_y = area.bottom().saturating_sub(1);
    let meter_y = detail_y.saturating_sub(1);
    let graph_top = area.y.saturating_add(1);
    let graph_height = meter_y.saturating_sub(graph_top);
    if graph_height > 0 {
        let graph_width = usize::from(area.width.saturating_sub(2));
        let retained = history.len().min(graph_width);
        let mut points = vec![0; graph_width.saturating_sub(retained)];
        points.extend_from_slice(&history[history.len().saturating_sub(retained)..]);
        if points.last().copied().unwrap_or_default() != u64::from(percent) {
            if points.len() == graph_width && !points.is_empty() {
                points.remove(0);
            }
            points.push(u64::from(percent));
        }
        frame.render_widget(
            Sparkline::default()
                .data(&points)
                .max(100)
                .style(active_style),
            Rect::new(
                area.x.saturating_add(1),
                graph_top,
                area.width.saturating_sub(2),
                graph_height,
            ),
        );
    }

    let meter_width = usize::from(area.width.saturating_sub(2));
    let filled = (usize::from(percent) * meter_width).div_ceil(100);
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let muted_style = palette.role(palette.muted, Modifier::DIM);
    for index in 0..meter_width {
        let Some(cell) = frame
            .buffer_mut()
            .cell_mut((area.x.saturating_add(1 + index as u16), meter_y))
        else {
            continue;
        };
        if index < filled {
            let segment_percent = ((index + 1) * 100) / meter_width.max(1);
            let segment_style = if segment_percent >= 90 {
                palette.role(palette.error, Modifier::BOLD)
            } else if segment_percent >= 70 {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.success, Modifier::BOLD)
            };
            cell.set_symbol(if unicode { "▪" } else { "#" })
                .set_style(segment_style);
        } else {
            cell.set_symbol(if unicode { "▫" } else { "." })
                .set_style(muted_style);
        }
    }

    if detail_y > area.y {
        frame.render_widget(
            Paragraph::new(detail)
                .alignment(Alignment::Center)
                .style(palette.role(palette.muted, Modifier::DIM)),
            Rect::new(area.x, detail_y, area.width, 1),
        );
    }
}

pub(crate) fn dense_telemetry_meter_supported(area: Rect) -> bool {
    area.width >= 12 && area.height >= 4
}

pub(crate) fn render_cpu_telemetry_cell(frame: &mut Frame, app: &App, area: Rect) {
    let Some(percent) = app
        .host_telemetry
        .cpu_utilization_percent
        .map(|value| value.min(100))
    else {
        render_cpu_gauge(frame, app, area);
        return;
    };
    if !dense_telemetry_meter_supported(area) {
        render_cpu_gauge(frame, app, area);
        return;
    }
    let detail = app
        .host_telemetry
        .logical_cpu_count
        .map_or_else(|| "utilization".into(), |cores| format!("{cores} cores"));
    let projection = app.host_telemetry_projection();
    render_dense_telemetry_meter(
        frame,
        app,
        area,
        DenseTelemetryMeter {
            title: "CPU Usage",
            percent,
            detail: &detail,
            active_style: cpu_meter_style(app, percent),
            history: &projection
                .series(TelemetryMetric::HostCpuUtilization)
                .history
                .points,
        },
    );
}

pub(crate) fn render_ram_telemetry_cell(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.host_telemetry.memory_total_bytes;
    let available = app.host_telemetry.memory_available_bytes;
    let (Some(percent), Some(total), Some(available)) =
        (utilization_percent(total, available), total, available)
    else {
        render_ram_gauge(frame, app, area);
        return;
    };
    if !dense_telemetry_meter_supported(area) {
        render_ram_gauge(frame, app, area);
        return;
    }
    let detail = format_bytes_pair(total - available, total);
    let projection = app.host_telemetry_projection();
    render_dense_telemetry_meter(
        frame,
        app,
        area,
        DenseTelemetryMeter {
            title: "RAM Usage",
            percent,
            detail: &detail,
            active_style: memory_meter_style(app, percent),
            history: &projection
                .series(TelemetryMetric::HostMemoryCapacity)
                .history
                .points,
        },
    );
}

pub(crate) fn render_build_filesystem_telemetry_cell(frame: &mut Frame, app: &App, area: Rect) {
    let total = app.host_telemetry.disk_total_bytes;
    let available = app.host_telemetry.disk_available_bytes;
    let (Some(percent), Some(_total), Some(available), Some(_build_dir)) = (
        utilization_percent(total, available),
        total,
        available,
        app.workspace.build_dir.as_deref(),
    ) else {
        render_disk_gauge(frame, app, area);
        return;
    };
    if !dense_telemetry_meter_supported(area) {
        render_disk_gauge(frame, app, area);
        return;
    }
    let detail = format!("{} free", format_bytes(available));
    let projection = app.host_telemetry_projection();
    render_dense_telemetry_meter(
        frame,
        app,
        area,
        DenseTelemetryMeter {
            title: "Build FS Usage",
            percent,
            detail: &detail,
            active_style: telemetry_meter_style(app, percent),
            history: &projection
                .series(TelemetryMetric::BuildFilesystemCapacity)
                .history
                .points,
        },
    );
}
