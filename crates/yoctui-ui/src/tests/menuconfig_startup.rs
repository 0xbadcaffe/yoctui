use super::*;

#[test]
fn menuconfig_preparation_renders_navigation_not_an_input_capturing_terminal() {
    let mut app = ux_terminal_render_fixture();
    app.screen = Screen::Kernel;
    let id = app.daemon.pty_sessions[0].id;
    app.daemon.pty_sessions[0].name = "kernel menuconfig".into();
    app.daemon.pty_details[0].kind = yoctui_model::ClientDaemonPtyKind::Menuconfig;
    app.kernel.menuconfig_terminal = yoctui_model::PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(id),
        foreground: true,
        ..yoctui_model::PlatformTerminalState::default()
    };
    app.daemon.pty_screens[0].application_cursor = false;
    let output = rendered_text(&app, 160, 50);
    assert!(output.contains("Starting Kernel menuconfig"), "{output}");
    assert!(
        output.contains("Menuconfig preparing in background"),
        "{output}"
    );
    assert!(output.contains("navigation available"), "{output}");
    assert!(!output.contains("Ctrl+G Yoctui"), "{output}");

    let _ = yoctui_model::update(&mut app, Action::Open(Screen::Tasks));
    app.daemon.pty_screens[0].application_cursor = true;
    let output = rendered_text(&app, 160, 50);
    assert_eq!(app.screen, Screen::Tasks);
    assert!(!output.contains("Ctrl+G Yoctui"), "{output}");
    assert!(!app.platform_menuconfig_waiting());
    assert_eq!(app.kernel.menuconfig_terminal.session_id, Some(id));
}
