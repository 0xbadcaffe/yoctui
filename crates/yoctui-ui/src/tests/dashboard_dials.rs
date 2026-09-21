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
