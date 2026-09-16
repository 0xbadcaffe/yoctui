//! Dashboard capacity bars, with explicit unavailable and accessible fallbacks.
use super::*;
use ratatui::widgets::Gauge;

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
        ("Sstate Reuse", None, "not reported".into(), 70),
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
        let bar = Rect::new(
            rows[1].x.saturating_add(2),
            rows[1].y + rows[1].height / 2,
            rows[1].width.saturating_sub(4),
            1,
        );
        frame.render_widget(
            Gauge::default()
                .gauge_style(
                    dashboard_meter_style(app, percent, warning_at).bg(palette.inactive_border),
                )
                .ratio(f64::from(percent) / 100.0)
                .use_unicode(true)
                .label(format!("{percent}%")),
            bar,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};
    #[test]
    fn dashboard_resource_bars_are_contiguous_exact_and_truthful() {
        let mut app = App::new(10, 1024);
        app.host_telemetry.cpu_utilization_percent = Some(72);
        app.host_telemetry.memory_total_bytes = Some(1000);
        app.host_telemetry.memory_available_bytes = Some(590);
        for width in [80, 100, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, 9)).unwrap();
            terminal
                .draw(|frame| render_dashboard_dials(frame, &app, frame.area()))
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(text.contains("72%"), "{text}");
            assert!(text.contains("41%"), "{text}");
            assert!(text.contains("unavailable"), "{text}");
            assert!(
                !text.chars().any(|c| ('\u{2800}'..='\u{28ff}').contains(&c)),
                "capacity bars must not use dotted arcs"
            );
        }
        for width in [80, 100] {
            app.color_enabled = false;
            app.preferences.symbols = SymbolPreference::Ascii;
            let mut terminal = Terminal::new(TestBackend::new(width, 9)).unwrap();
            terminal
                .draw(|frame| render_dashboard_dials(frame, &app, frame.area()))
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|c| c.symbol())
                .collect::<String>();
            assert!(text.contains("72%"), "{text}");
        }
    }
}
