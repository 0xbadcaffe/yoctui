//! Dashboard capacity dials, with explicit unavailable and accessible fallbacks.
use super::*;
use ratatui::widgets::canvas::{Canvas, Points};

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
            format_bytes_pair(total - available, total)
        }
        _ => "unavailable".into(),
    };
    let values = [
        (
            "CPU Usage",
            cpu,
            telemetry.logical_cpu_count.map_or_else(
                || "utilization".into(),
                |cores| format!("{cores} logical cores"),
            ),
        ),
        (
            "RAM Usage",
            ram,
            pair(
                telemetry.memory_total_bytes,
                telemetry.memory_available_bytes,
            ),
        ),
        (
            "Build FS Usage",
            disk,
            pair(telemetry.disk_total_bytes, telemetry.disk_available_bytes),
        ),
        ("Sstate Reuse", None, "backend does not report".into()),
    ];
    let cells = Layout::horizontal([Constraint::Ratio(1, 4); 4]).split(inner);
    for (index, ((title, percent, detail), cell)) in
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
            for radius in [0.82, 0.88, 0.94, 1.0] {
                let point = (radius * angle.cos(), radius * angle.sin());
                if percent > 0 && step * 100 <= i32::from(percent) * 180 {
                    active.push(point);
                } else {
                    inactive.push(point);
                }
            }
        }
        let color = telemetry_meter_style(app, percent)
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
