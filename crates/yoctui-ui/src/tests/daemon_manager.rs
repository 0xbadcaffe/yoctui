use super::*;

#[test]
fn daemon_manager_renders_health_logs_review_and_configuration_at_boundary_sizes() {
    let mut app = App::new(100, 100_000);
    app.screen = Screen::Daemons;
    app.focus = FocusTarget::Workspace;
    app.daemon_manager.details = vec![
        "Host: demo-host".into(),
        "Local IP addresses: 192.0.2.1".into(),
    ];
    app.daemon_manager.build_directory = "/build".into();
    app.daemon_manager.source_directory = "/source".into();
    app.daemon_manager.logs = vec!["daemon discovery: build identity ready".into()];
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Host: demo-host"), "{width}x{height}: {text}");
        app.daemon_manager.logs_visible = true;
        assert!(rendered_text(&app, width, height).contains("daemon discovery:"));
        app.daemon_manager.logs_visible = false;
        app.daemon_manager.editing = true;
        assert!(rendered_text(&app, width, height).contains("Configure local daemon"));
        app.daemon_manager.editing = false;
        app.daemon_manager.review = Some(yoctui_model::DaemonControl::Stop);
        let text = rendered_text(&app, width, height);
        assert!(text.contains("Daemon control review"));
        assert!(text.contains("Esc cancel"));
        app.daemon_manager.review = None;
    }
}
