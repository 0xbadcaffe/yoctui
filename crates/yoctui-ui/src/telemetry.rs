//! Telemetry threshold styles and segmented dot-meter rendering.

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

pub(super) fn cpu_meter_style(app: &App, percent: u8) -> Style {
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

pub(super) fn render_dot_meter(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    percent: u8,
    label: String,
    label_style: Style,
) {
    if area.is_empty() {
        return;
    }
    let percent = percent.min(100);
    let palette = ThemePalette::for_app(app);
    let filled = (usize::from(percent) * usize::from(area.width)).div_ceil(100);
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    for index in 0..usize::from(area.width) {
        let Some(cell) = frame
            .buffer_mut()
            .cell_mut((area.x.saturating_add(index as u16), area.y))
        else {
            continue;
        };
        if index < filled {
            let segment_percent = ((index + 1) * 100) / usize::from(area.width).max(1);
            let style = if segment_percent >= 90 {
                palette.role(palette.error, Modifier::BOLD)
            } else if segment_percent >= 70 {
                palette.role(palette.warning, Modifier::BOLD)
            } else {
                palette.role(palette.success, Modifier::BOLD)
            };
            cell.set_symbol(if unicode { "▪" } else { "#" })
                .set_style(style);
        } else {
            cell.set_symbol(if unicode { "▫" } else { "." })
                .set_style(palette.role(palette.muted, Modifier::DIM));
        }
    }
    frame.render_widget(
        Paragraph::new(label)
            .alignment(Alignment::Center)
            .style(label_style.add_modifier(Modifier::BOLD)),
        area,
    );
}

pub(super) fn render_cpu_gauge(frame: &mut Frame, app: &App, area: Rect) {
    if area.is_empty() {
        return;
    }
    let palette = ThemePalette::for_app(app);
    if let Some(cpu) = app.host_telemetry.cpu_utilization_percent {
        let cpu = cpu.min(100);
        render_dot_meter(
            frame,
            app,
            area,
            cpu,
            cpu_gauge_label(cpu, app.host_telemetry.logical_cpu_count, area.width),
            cpu_meter_style(app, cpu),
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
