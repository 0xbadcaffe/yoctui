//! Dashboard capacity dials, with explicit unavailable and accessible fallbacks.
use super::*;
use ratatui::widgets::canvas::{Canvas, Points};

fn dashboard_cpu_context(percent: Option<u8>, cores: Option<u16>) -> String {
    match (percent, cores) {
        (Some(percent), Some(cores)) => format!(
            "{:.2} / {:.2} cores",
            f64::from(percent.min(100)) * f64::from(cores) / 100.0,
            f64::from(cores)
        ),
        (_, Some(cores)) => format!("{cores} logical cores"),
        _ => "utilization".into(),
    }
}

fn dashboard_meter_style(app: &App, percent: u8, warning_at: u8) -> Style {
    let palette = ThemePalette::for_app(app);
    if percent >= 90 {
        palette.role(palette.error, Modifier::BOLD)
    } else if percent >= warning_at {
        palette.role(palette.warning, Modifier::BOLD)
    } else {
        palette.role(palette.progress, Modifier::BOLD)
    }
}

pub(super) fn render_dashboard_dials(frame: &mut Frame, app: &App, area: Rect) {
    if area.width < 64
        || area.height < 7
        || !app.color_enabled
        || app.preferences.symbols == SymbolPreference::Ascii
    {
        render_telemetry_strip(frame, app, area);
        return;
    }
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Resource Telemetry", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let telemetry = &app.host_telemetry;
    let cpu = telemetry
        .cpu_utilization_percent
        .map(|value| value.min(100));
    let ram = utilization_percent(
        telemetry.memory_total_bytes,
        telemetry.memory_available_bytes,
    );
    let disk = app.workspace.build_dir.as_ref().and_then(|_| {
        utilization_percent(telemetry.disk_total_bytes, telemetry.disk_available_bytes)
    });
    let pair = |total: Option<u64>, available: Option<u64>| match (total, available) {
        (Some(total), Some(available)) if total > 0 && available <= total => {
            format_bytes_pair_with(total - available, total, 2, " / ")
        }
        _ => "unavailable".into(),
    };
    let values = [
        (
            "CPU Usage",
            cpu,
            dashboard_cpu_context(cpu, telemetry.logical_cpu_count),
            70,
        ),
        (
            "RAM Usage",
            ram,
            pair(
                telemetry.memory_total_bytes,
                telemetry.memory_available_bytes,
            ),
            80,
        ),
        (
            "Build FS Usage",
            disk,
            pair(telemetry.disk_total_bytes, telemetry.disk_available_bytes),
            60,
        ),
        ("Sstate Reuse", None, "backend does not report".into(), 70),
    ];
    let cells = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(inner);
    for (index, ((title, percent, detail, warning_at), cell)) in
        values.into_iter().zip(cells.iter().copied()).enumerate()
    {
        let divider = Block::default()
            .borders(if index < 3 {
                Borders::RIGHT
            } else {
                Borders::NONE
            })
            .border_style(palette.role(palette.inactive_border, Modifier::empty()));
        let body = divider.inner(cell);
        frame.render_widget(divider, cell);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(body);
        frame.render_widget(Paragraph::new(title).alignment(Alignment::Center), rows[0]);
        frame.render_widget(
            Paragraph::new(bounded_cell_text(&detail, rows[2].width)).alignment(Alignment::Center),
            rows[2],
        );
        let Some(percent) = percent else {
            let label = Rect::new(rows[1].x, rows[1].y + rows[1].height / 2, rows[1].width, 1);
            frame.render_widget(
                Paragraph::new("unavailable")
                    .alignment(Alignment::Center)
                    .style(palette.role(palette.disabled, Modifier::empty())),
                label,
            );
            continue;
        };
        let mut active = Vec::new();
        let mut inactive = Vec::new();
        for step in 0..=180 {
            let angle = std::f64::consts::PI * (1.0 - f64::from(step) / 180.0);
            let point = (angle.cos(), angle.sin());
            if percent > 0 && step * 100 <= i32::from(percent) * 180 {
                active.push(point);
            } else {
                inactive.push(point);
            }
        }
        let color = dashboard_meter_style(app, percent, warning_at)
            .fg
            .unwrap_or(palette.progress);
        frame.render_widget(
            Canvas::default()
                .background_color(palette.background)
                .marker(ratatui::symbols::Marker::Braille)
                .x_bounds([-1.15, 1.15])
                .y_bounds([0.0, 1.15])
                .paint(|context| {
                    context.draw(&Points {
                        coords: &inactive,
                        color: palette.muted,
                    });
                    context.draw(&Points {
                        coords: &active,
                        color,
                    });
                }),
            rows[1],
        );
        let label = Rect::new(
            rows[1].x,
            rows[1].bottom().saturating_sub(1),
            rows[1].width,
            1,
        );
        frame.render_widget(
            Paragraph::new(format!("{percent}%"))
                .alignment(Alignment::Center)
                .style(palette.role(color, Modifier::BOLD)),
            label,
        );
    }
}
