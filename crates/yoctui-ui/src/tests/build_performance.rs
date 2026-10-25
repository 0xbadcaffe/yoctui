use super::*;
#[test]
fn performance_report_scrolls_to_last_line_and_back_at_every_supported_size() {
    for (w, h) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let mut app = App::new(100, 4096);
        app.screen = Screen::BuildPerformance;
        app.focus = FocusTarget::Workspace;
        app.build_performance.visible_rows = usize::from(h - 7);
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| crate::build_performance_render::workspace(frame, &app, frame.area()))
            .unwrap();
        let text = |terminal: &Terminal<TestBackend>| {
            terminal
                .backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
        };
        assert!(text(&terminal).contains("HOST RESOURCES"));
        update(
            &mut app,
            Action::BuildPerformance(yoctui_model::BuildPerformanceAction::Last),
        );
        terminal
            .draw(|frame| crate::build_performance_render::workspace(frame, &app, frame.area()))
            .unwrap();
        assert!(text(&terminal).contains("Cache checks show configuration"));
        update(
            &mut app,
            Action::BuildPerformance(yoctui_model::BuildPerformanceAction::First),
        );
        terminal
            .draw(|frame| crate::build_performance_render::workspace(frame, &app, frame.area()))
            .unwrap();
        assert!(text(&terminal).contains("HOST RESOURCES"));
    }
}
