//! Telemetry strip.
use super::*;

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

pub(crate) fn telemetry_available(app: &App) -> bool {
    let projection = app.host_telemetry_projection();
    projection
        .series
        .iter()
        .any(TelemetrySeriesProjection::is_supported)
        || (app.workspace.build_dir.is_some()
            && utilization_percent(
                app.host_telemetry.disk_total_bytes,
                app.host_telemetry.disk_available_bytes,
            )
            .is_some())
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
        TelemetryCell::Cpu => render_cpu_telemetry_cell(frame, app, inner),
        TelemetryCell::Ram => render_ram_telemetry_cell(frame, app, inner),
        TelemetryCell::BuildFilesystem => {
            render_build_filesystem_telemetry_cell(frame, app, inner);
        }
        TelemetryCell::Sstate => {
            let palette = ThemePalette::for_app(app);
            frame.render_widget(
                Paragraph::new("SSTATE ! unavailable")
                    .style(palette.role(palette.disabled, Modifier::DIM)),
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
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    for ((label, percent), cell) in values.into_iter().zip(cells.iter()) {
        let text = percent.map_or_else(
            || format!("{label} --"),
            |value| format!("{label} {value}%"),
        );
        frame.render_widget(
            Paragraph::new(text).style(palette.role(palette.informational, Modifier::BOLD)),
            Rect::new(cell.x, cell.y, cell.width, 1),
        );
        if cell.height < 2 {
            continue;
        }
        let bar = Rect::new(cell.x, cell.y + 1, cell.width.saturating_sub(1), 1);
        let Some(percent) = percent else {
            frame.render_widget(
                Paragraph::new("unavailable").style(palette.role(palette.muted, Modifier::DIM)),
                bar,
            );
            continue;
        };
        let filled = (u32::from(percent) * u32::from(bar.width)).div_ceil(100);
        for index in 0..bar.width {
            let active = u32::from(index) < filled;
            let segment = u32::from(index + 1) * 100 / u32::from(bar.width.max(1));
            let color = if !active {
                palette.muted
            } else if segment >= 90 {
                palette.error
            } else if segment >= 70 {
                palette.warning
            } else {
                palette.success
            };
            let symbol = match (unicode, active) {
                (true, true) => "▪",
                (true, false) => "▫",
                (false, true) => "#",
                (false, false) => ".",
            };
            if let Some(cell) = frame.buffer_mut().cell_mut((bar.x + index, bar.y)) {
                cell.set_symbol(symbol).set_style(palette.role(
                    color,
                    if active {
                        Modifier::BOLD
                    } else {
                        Modifier::DIM
                    },
                ));
            }
        }
    }
}

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
