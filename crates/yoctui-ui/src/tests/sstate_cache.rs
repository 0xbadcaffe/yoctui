use super::*;
#[test]
fn sstate_report_and_review_render_at_supported_sizes() {
    for (w, h) in [(80, 24), (100, 30), (160, 48)] {
        let mut app = App::new(100, 4096);
        app.screen = Screen::Maintenance;
        app.focus = FocusTarget::Workspace;
        app.sstate_cache.dashboard = true;
        app.sstate_cache.visible_rows = usize::from(h - 7);
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        let text = |t: &Terminal<TestBackend>| {
            t.backend()
                .buffer()
                .content()
                .iter()
                .map(|c| c.symbol())
                .collect::<String>()
        };
        terminal
            .draw(|f| crate::sstate_cache_render::workspace(f, &app, f.area()))
            .unwrap();
        assert!(text(&terminal).contains("SSTATE REUSE"));
        update(
            &mut app,
            Action::SstateCache(yoctui_model::SstateCacheAction::Last),
        );
        terminal
            .draw(|f| crate::sstate_cache_render::workspace(f, &app, f.area()))
            .unwrap();
        assert!(text(&terminal).contains("Tool availability"));
        let profile = yoctui_model::SstateCacheProfile {
            build_directory: "/build".into(),
            settings: yoctui_model::SstateCacheSettings {
                hash_server: "auto".into(),
                ..Default::default()
            },
        };
        terminal
            .draw(|f| crate::sstate_cache_render::review(f, &app, &profile, f.area()))
            .unwrap();
        assert!(text(&terminal).contains("Enter Save profile"));
        assert!(text(&terminal).contains("local.conf is unchanged"));
    }
}
#[test]
fn sstate_footer_matches_the_visible_report_and_preserves_tools() {
    let mut app = App::new(100, 4096);
    app.screen = Screen::Maintenance;
    app.focus = FocusTarget::Workspace;
    app.sstate_cache.dashboard = true;
    let report = crate::footer::footer_shortcuts(&app);
    assert!(report.contains("e configure"));
    assert!(report.contains("t tools"));
    assert!(!report.contains("Enter inspect"));
    app.sstate_cache.dashboard = false;
    let tools = crate::footer::footer_shortcuts(&app);
    assert!(tools.contains("Enter inspect"));
    assert!(tools.contains("p cache performance"));
}
