//! Telemetry threshold styles and CPU gauge rendering.

use super::*;

pub(super) fn telemetry_meter_style(app: &App, percent: u8) -> Style {
    let palette = ThemePalette::for_app(app);
    if percent >= 90 {
        palette.role(palette.error, Modifier::BOLD)
    } else if percent >= 70 {
        palette.role(palette.warning, Modifier::BOLD)
    } else {
        palette.role(palette.progress, Modifier::BOLD)
    }
}

fn cpu_meter_style(app: &App, percent: u8) -> Style {
    let palette = ThemePalette::for_app(app);
    if percent >= 90 {
        palette.role(palette.error, Modifier::BOLD)
    } else if percent >= 70 {
        palette.role(palette.warning, Modifier::BOLD)
    } else {
        palette.role(palette.graph_cpu, Modifier::BOLD)
    }
}

fn cpu_gauge_label(cpu: u8, cores: Option<u16>, width: u16) -> String {
    match (cores, width) {
        (Some(cores), 28..) => format!("CPU {cpu:>3}% · {cores} cores"),
        (Some(cores), 16..) => format!("CPU {cpu}% · {cores}c"),
        (None, 28..) => format!("CPU {cpu:>3}%"),
        _ => format!("CPU {cpu}%"),
    }
}

pub(super) fn render_cpu_gauge(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    if let Some(cpu) = app.host_telemetry.cpu_utilization_percent {
        let cpu = cpu.min(100);
        frame.render_widget(
            Gauge::default()
                .ratio(f64::from(cpu) / 100.0)
                .label(cpu_gauge_label(
                    cpu,
                    app.host_telemetry.logical_cpu_count,
                    area.width,
                ))
                .gauge_style(cpu_meter_style(app, cpu)),
            area,
        );
    } else {
        frame.render_widget(
            Paragraph::new("CPU ! unavailable")
                .style(palette.role(palette.disabled, Modifier::DIM)),
            area,
        );
    }
}

pub(super) fn memory_meter_style(app: &App, percent: u8) -> Style {
    let palette = ThemePalette::for_app(app);
    if percent >= 90 {
        palette.role(palette.error, Modifier::BOLD)
    } else if percent >= 80 {
        palette.role(palette.warning, Modifier::BOLD)
    } else {
        palette.role(palette.graph_memory, Modifier::BOLD)
    }
}
