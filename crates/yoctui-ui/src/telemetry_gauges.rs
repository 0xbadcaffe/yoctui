//! Telemetry gauges.
use super::*;

pub(crate) fn format_bytes_pair_with(
    used: u64,
    total: u64,
    precision: usize,
    separator: &str,
) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut divisor = 1_u64;
    let mut unit = 0;
    while total / divisor >= 1024 && unit < UNITS.len() - 1 {
        let Some(next) = divisor.checked_mul(1024) else {
            break;
        };
        divisor = next;
        unit += 1;
    }
    if unit == 0 {
        format!("{used}{separator}{total} {}", UNITS[unit])
    } else {
        format!(
            "{:.precision$}{separator}{:.precision$} {}",
            used as f64 / divisor as f64,
            total as f64 / divisor as f64,
            UNITS[unit]
        )
    }
}

pub(crate) fn format_bytes_pair(used: u64, total: u64) -> String {
    format_bytes_pair_with(used, total, 1, "/")
}

pub(crate) fn ram_gauge_label(percent: u8, used: u64, total: u64, width: u16) -> String {
    if width >= 38 {
        format!(
            "RAM {percent:>3}% · {} / {}",
            format_bytes(used),
            format_bytes(total)
        )
    } else if width >= 28 {
        format!("RAM {percent}% · {}", format_bytes_pair(used, total))
    } else {
        format!("RAM {percent}%")
    }
}

pub(crate) fn render_ram_gauge(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let total = app.host_telemetry.memory_total_bytes;
    let available = app.host_telemetry.memory_available_bytes;
    if let (Some(percent), Some(total), Some(available)) =
        (utilization_percent(total, available), total, available)
    {
        let used = total - available;
        render_dot_meter(
            frame,
            app,
            area,
            percent,
            ram_gauge_label(percent, used, total, area.width),
            memory_meter_style(app, percent),
        );
    } else {
        frame.render_widget(
            Paragraph::new("RAM ! unavailable")
                .style(palette.role(palette.disabled, Modifier::DIM)),
            area,
        );
    }
}

pub(crate) fn disk_gauge_label(
    percent: u8,
    available: u64,
    total: u64,
    build_dir: &std::path::Path,
    width: u16,
) -> String {
    if width >= 52 {
        format!(
            "BUILD FS {percent:>3}% · {} free · {}",
            format_bytes_pair(available, total),
            build_dir.display()
        )
    } else if width >= 34 {
        format!(
            "BUILD FS {percent}% · {} free",
            format_bytes_pair(available, total)
        )
    } else if width >= 16 {
        format!("BUILD FS {percent}%")
    } else {
        format!("FS {percent}%")
    }
}

pub(crate) fn render_disk_gauge(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let total = app.host_telemetry.disk_total_bytes;
    let available = app.host_telemetry.disk_available_bytes;
    let build_dir = app.workspace.build_dir.as_deref();
    if let (Some(percent), Some(total), Some(available), Some(build_dir)) = (
        utilization_percent(total, available),
        total,
        available,
        build_dir,
    ) {
        render_dot_meter(
            frame,
            app,
            area,
            percent,
            disk_gauge_label(percent, available, total, build_dir, area.width),
            telemetry_meter_style(app, percent),
        );
    } else {
        frame.render_widget(
            Paragraph::new("BUILD FS ! unavailable")
                .style(palette.role(palette.disabled, Modifier::DIM)),
            area,
        );
    }
}

pub(crate) fn format_rate(bytes_per_second: u64) -> String {
    format!("{}/s", format_bytes(bytes_per_second))
}

pub(crate) struct RateHistory<'a> {
    pub(crate) label: &'a str,
    pub(crate) compact_label: &'a str,
    pub(crate) series: &'a TelemetrySeriesProjection,
}

pub(crate) fn render_rate_history(frame: &mut Frame, app: &App, area: Rect, rate: RateHistory<'_>) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let label = if area.width >= 28 {
        rate.label
    } else {
        rate.compact_label
    };
    let text = rate.series.history.current.map_or_else(
        || format!("{label} ! unavailable"),
        |rate| format!("{label} {}", format_rate(rate)),
    );
    let text_style = if rate.series.history.current.is_some() {
        palette.widget_styles().role(rate.series.history.role)
    } else {
        palette.role(palette.disabled, Modifier::DIM)
    };
    if area.height >= 2 {
        let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
        frame.render_widget(Paragraph::new(text).style(text_style), rows[0]);
        frame.render_widget(
            Sparkline::default()
                .data(&rate.series.history.points)
                .max(
                    rate.series
                        .history
                        .points
                        .iter()
                        .copied()
                        .max()
                        .unwrap_or(1)
                        .max(1),
                )
                .style(palette.widget_styles().role(rate.series.history.role)),
            rows[1],
        );
        return;
    }
    if area.width < 18 {
        frame.render_widget(Paragraph::new(text).style(text_style), area);
        return;
    }
    let label_width = if area.width >= 28 { 20 } else { 14 };
    let columns = Layout::horizontal([
        Constraint::Length(label_width.min(area.width)),
        Constraint::Min(1),
    ])
    .split(area);
    frame.render_widget(Paragraph::new(text).style(text_style), columns[0]);
    frame.render_widget(
        Sparkline::default()
            .data(&rate.series.history.points)
            .max(
                rate.series
                    .history
                    .points
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(1)
                    .max(1),
            )
            .style(palette.widget_styles().role(rate.series.history.role)),
        columns[1],
    );
}

pub(crate) fn render_disk_io_projection(
    frame: &mut Frame,
    app: &App,
    projection: &yoctui_model::HostTelemetryProjection,
    read_area: Rect,
    write_area: Rect,
) {
    let read = projection.series(TelemetryMetric::DiskReadRate);
    render_rate_history(
        frame,
        app,
        read_area,
        RateHistory {
            label: "Read",
            compact_label: "R",
            series: read,
        },
    );
    let write = projection.series(TelemetryMetric::DiskWriteRate);
    render_rate_history(
        frame,
        app,
        write_area,
        RateHistory {
            label: "Write",
            compact_label: "W",
            series: write,
        },
    );
}

#[cfg(test)]
pub(crate) fn render_disk_io(frame: &mut Frame, app: &App, read_area: Rect, write_area: Rect) {
    let projection = app.host_telemetry_projection();
    render_disk_io_projection(frame, app, &projection, read_area, write_area);
}

pub(crate) fn render_network_io_projection(
    frame: &mut Frame,
    app: &App,
    projection: &yoctui_model::HostTelemetryProjection,
    receive_area: Rect,
    transmit_area: Rect,
) {
    let receive = projection.series(TelemetryMetric::NetworkReceiveRate);
    render_rate_history(
        frame,
        app,
        receive_area,
        RateHistory {
            label: "RX",
            compact_label: "RX",
            series: receive,
        },
    );
    let transmit = projection.series(TelemetryMetric::NetworkTransmitRate);
    render_rate_history(
        frame,
        app,
        transmit_area,
        RateHistory {
            label: "TX",
            compact_label: "TX",
            series: transmit,
        },
    );
}

#[cfg(test)]
pub(crate) fn render_network_io(
    frame: &mut Frame,
    app: &App,
    receive_area: Rect,
    transmit_area: Rect,
) {
    let projection = app.host_telemetry_projection();
    render_network_io_projection(frame, app, &projection, receive_area, transmit_area);
}
