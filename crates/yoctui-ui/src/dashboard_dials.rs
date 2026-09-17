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
    if area.width < 64 || area.height < 8 {
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
    ];
    let sections = Layout::vertical([Constraint::Length(3), Constraint::Length(3)]).split(inner);
    let cells = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(sections[0]);
    frame.render_widget(
        Paragraph::new(
            app.cache_status_lines()
                .into_iter()
                .map(Line::from)
                .collect::<Vec<_>>(),
        ),
        sections[1],
    );
    for (index, ((title, percent, detail, warning_at), cell)) in
        values.into_iter().zip(cells.iter().copied()).enumerate()
    {
        let divider = Block::default()
            .borders(if index < 2 {
                Borders::RIGHT
            } else {
                Borders::NONE
            })
            .border_style(palette.role(palette.inactive_border, Modifier::empty()));
        let body = divider.inner(cell);
        frame.render_widget(divider, cell);
        let rows = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
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
        if app.preferences.symbols == SymbolPreference::Ascii {
            let width = usize::from(bar.width.saturating_sub(7));
            let filled = usize::from(percent) * width / 100;
            frame.render_widget(
                Paragraph::new(format!(
                    "[{}{}] {percent}%",
                    "#".repeat(filled),
                    "-".repeat(width - filled)
                )),
                bar,
            );
            continue;
        }
        frame.render_widget(
            Gauge::default()
                .gauge_style(
                    dashboard_meter_style(app, percent, warning_at).bg(palette.inactive_border),
                )
                .ratio(f64::from(percent) / 100.0)
                .use_unicode(app.preferences.symbols != SymbolPreference::Ascii)
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
    fn cache_dashboard_shows_authoritative_counts_without_offline_promise() {
        let mut app = App::new(10, 1024);
        app.build.cache.summary = Some(yoctui_model::SstateSummary {
            wanted: 10,
            local: 3,
            mirrors: 2,
            missed: 5,
            current: 8,
        });
        app.build.cache.fetch_completed = 1234;
        app.build.cache.fetch_failed = 2;
        for width in [64, 80, 160] {
            let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
            terminal
                .draw(|frame| render_dashboard_dials(frame, &app, frame.area()))
                .unwrap();
            let text = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect::<String>();
            assert!(text.contains("3 local + 2 mirrors / 10 wanted"), "{text}");
            assert!(text.contains("1234 completed, 2 failed"), "{text}");
            assert!(text.contains("offline readiness: unverified"), "{text}");
        }
    }

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
            assert!(text.contains("Downloads:"), "{text}");
            assert!(text.contains("offline readiness: unverified"), "{text}");
            let buffer = terminal.backend().buffer();
            assert!(
                (1..width - 1).any(|x| buffer[(x, 2)].symbol() == "7"),
                "bar must directly follow the title"
            );
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
